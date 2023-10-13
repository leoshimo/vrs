//! A fiber of execution that can be cooperatively scheduled via yielding.
use tracing::{debug, warn};

use super::{Env, Inst};
use crate::{compile, parse, Error, Lambda, NativeFn, Result, Val};
use std::{cell::RefCell, rc::Rc};

// TODO: Revisit Fiber API Surface
// - start should return Result<???>
// - run should return Result<???>
// - Status can be pruned
// - run loop can be refactored

// TODO: Refactor - Fiber Internals for `run`

#[derive(Debug)]
pub struct Fiber {
    cframes: Vec<CallFrame>,
    stack: Vec<Val>,
    status: Status,
    global: Rc<RefCell<Env>>,
}

/// Status for a fiber
#[derive(Debug, PartialEq)]
pub enum Status {
    New,
    Running,
    Yielded,
    Completed(Result<Val>),
}

impl Status {
    pub fn unwrap(&self) -> &Val {
        match self {
            Status::Completed(v) => v.as_ref().unwrap(),
            _ => panic!("Status is not completed!"),
        }
    }
}

/// Single call frame
#[derive(Debug)]
struct CallFrame {
    ip: usize,
    code: Vec<Inst>,
    env: Rc<RefCell<Env>>,
}

/// Start execution of a fiber
pub fn start(f: &mut Fiber) -> Result<&Status> {
    if f.status != Status::New {
        return Err(Error::AlreadyRunning);
    }
    Ok(run(f))
}

/// Run the fiber until it completes or yields
fn run(f: &mut Fiber) -> &Status {
    f.status = Status::Running;
    f.status = 'outer: loop {
        let inst = f.inst().clone(); // TODO(opt): Defer cloning until args need cloning
        f.top_mut().ip += 1;

        // TODO(dev): Add fiber debug flag
        debug!(
            "frame {} ip {}: \n\t{:?}\n\t{:?}",
            f.cframes.len() - 1,
            f.top().ip,
            inst,
            f.stack,
        );

        match inst {
            Inst::PushConst(form) => {
                f.stack.push(form);
            }
            Inst::DefSym(s) => match f.stack.last() {
                Some(value) => f.top().env.borrow_mut().define(&s, value.clone()),
                None => {
                    break Status::Completed(Err(Error::UnexpectedStack(
                        "Expected stack to be nonempty".to_string(),
                    )))
                }
            },
            Inst::SetSym(s) => match f.stack.last() {
                Some(value) => {
                    if let Err(e) = f.top().env.borrow_mut().set(&s, value.clone()) {
                        break Status::Completed(Err(e));
                    }
                }
                None => {
                    break Status::Completed(Err(Error::UnexpectedStack(
                        "Expected stack to be nonempty".to_string(),
                    )))
                }
            },
            Inst::GetSym(s) => match f.top().env.clone().borrow().get(&s) {
                Some(value) => f.stack.push(value.clone()),
                None => {
                    break Status::Completed(Err(Error::UndefinedSymbol(s)));
                }
            },
            Inst::MakeFunc => {
                // TODO(ref): Break makes error prop verbose
                let code = match f.stack.pop() {
                    Some(Val::Bytecode(b)) => b,
                    _ => {
                        break Status::Completed(Err(Error::UnexpectedStack(
                            "Missing function bytecode".to_string(),
                        )))
                    }
                };
                let params = match f.stack.pop() {
                    Some(Val::List(p)) => p,
                    _ => {
                        break Status::Completed(Err(Error::UnexpectedStack(
                            "Missing parameter list".to_string(),
                        )))
                    }
                };
                let params = params
                    .into_iter()
                    .map(|f| match f {
                        Val::Symbol(s) => Ok(s),
                        _ => Err(Error::UnexpectedStack(
                            "Unexpected parameter list".to_string(),
                        )),
                    })
                    .collect::<Result<Vec<_>>>();
                let params = match params {
                    Ok(p) => p,
                    Err(e) => break Status::Completed(Err(e)),
                };

                f.stack.push(Val::Lambda(Lambda {
                    params,
                    code,
                    env: Rc::clone(&f.top().env),
                }));
            }
            Inst::CallFunc(nargs) => {
                let mut args = vec![];
                for _ in 0..nargs {
                    let v = match f.stack.pop() {
                        Some(v) => v,
                        None => {
                            break 'outer Status::Completed(Err(Error::UnexpectedStack(
                                "Missing expected {nargs} args".to_string(),
                            )))
                        }
                    };
                    args.push(v);
                }
                let args = args.into_iter().rev();
                match f.stack.pop() {
                    Some(Val::NativeFn(n)) => match (n.func)(&args.collect::<Vec<_>>()) {
                        Ok(v) => f.stack.push(v),
                        Err(e) => break Status::Completed(Err(e)),
                    },
                    Some(Val::Lambda(l)) => {
                        let mut fn_env = Env::extend(&l.env);
                        for (s, arg) in l.params.iter().zip(args) {
                            fn_env.define(s, arg);
                        }

                        f.cframes.push(CallFrame::from_bytecode(
                            Rc::new(RefCell::new(fn_env)),
                            l.code,
                        ))
                    }
                    _ => {
                        break Status::Completed(Err(Error::UnexpectedStack(
                            "Missing function object".to_string(),
                        )))
                    }
                };
            }
            Inst::PopTop => {
                if f.stack.pop().is_none() {
                    break Status::Completed(Err(Error::UnexpectedStack(
                        "Attempting to pop empty stack".to_string(),
                    )));
                }
            }
        }

        // end of func
        if f.status == Status::Running {
            while f.cframes.len() > 1 && f.top().is_done() {
                f.cframes.pop();
            }
            if f.at_end() {
                let res = f.stack.pop().expect("Stack should contain result");
                if !f.stack.is_empty() {
                    warn!("Fiber terminated with nonempty stack {:?}", f.stack);
                }
                break Status::Completed(Ok(res));
            }
        }
    };

    &f.status
}

impl Fiber {
    /// Create a new fiber from given bytecode
    pub fn from_bytecode(bytecode: Vec<Inst>) -> Self {
        let global = Rc::new(RefCell::new(Env::default()));
        Fiber {
            stack: vec![],
            cframes: vec![CallFrame::from_bytecode(Rc::clone(&global), bytecode)],
            status: Status::New,
            global,
        }
    }

    /// Create a new fiber from value
    pub fn from_val(val: &Val) -> Result<Self> {
        let bytecode = compile(val)?;
        Ok(Fiber::from_bytecode(bytecode))
    }

    /// Create a new fiber from given expressino
    pub fn from_expr(expr: &str) -> Result<Self> {
        let val: Val = parse(expr)?.into();
        Fiber::from_val(&val)
    }

    /// Retrieve status of fiber
    pub fn status(&self) -> &Status {
        &self.status
    }

    /// Check if stack is empty
    pub fn is_stack_empty(&self) -> bool {
        self.stack.is_empty()
    }

    /// Bind native function to global environment
    pub fn bind(&mut self, nativefn: NativeFn) -> &mut Self {
        self.global.borrow_mut().bind(nativefn);
        self
    }

    /// Reference to top of callstack
    fn top(&self) -> &CallFrame {
        self.cframes.last().expect("Fiber has no callframes!")
    }

    /// Reference to top of callstack
    fn top_mut(&mut self) -> &mut CallFrame {
        self.cframes.last_mut().expect("Fiber has no callframes!")
    }

    /// Get the current instruction
    fn inst(&self) -> &Inst {
        let top = self.top();
        let inst = top
            .code
            .get(top.ip)
            .expect("Callstack executing outside bytecode length");
        inst
    }

    /// Exhausted all call frames
    fn at_end(&self) -> bool {
        self.cframes.len() == 1 && self.cframes.last().unwrap().is_done()
    }
}

impl CallFrame {
    /// Create a new callframe for executing given bytecode from start
    pub fn from_bytecode(env: Rc<RefCell<Env>>, code: Vec<Inst>) -> Self {
        Self { ip: 0, env, code }
    }

    /// Whether or not call frame should return (i.e. on last call frame execution)
    pub fn is_done(&self) -> bool {
        self.ip == self.code.len()
    }
}

#[cfg(test)]
mod tests {
    use super::Inst::*;
    use super::*;
    use crate::SymbolId;
    use assert_matches::assert_matches;
    use tracing_test::traced_test;

    #[test]
    #[traced_test]
    fn fiber_load_const_return() {
        let mut f = Fiber::from_bytecode(vec![PushConst(Val::Int(5))]);
        start(&mut f).expect("should start");
        assert_eq!(f.status, Status::Completed(Ok(Val::Int(5))));

        let mut f = Fiber::from_bytecode(vec![PushConst(Val::string("Hi"))]);
        start(&mut f).expect("should start");
        assert_eq!(f.status, Status::Completed(Ok(Val::string("Hi"))));

        assert!(!logs_contain("ERROR"));
        assert!(!logs_contain("WARN"));
    }

    #[test]
    #[traced_test]
    fn def_symbol() {
        // fiber for (def x 5)
        let mut f = Fiber::from_bytecode(vec![PushConst(Val::Int(5)), DefSym(SymbolId::from("x"))]);

        start(&mut f).expect("should start");

        assert_eq!(
            f.top().env.borrow().get(&SymbolId::from("x")),
            Some(Val::Int(5)),
            "symbol should be defined in env"
        );
        assert_eq!(
            f.status,
            Status::Completed(Ok(Val::Int(5))),
            "should complete with value of stored symbol"
        );

        assert!(!logs_contain("ERROR"));
        assert!(!logs_contain("WARN"));
    }

    #[test]
    #[traced_test]
    fn get_symbol() {
        let mut f = Fiber::from_bytecode(vec![GetSym(SymbolId::from("x"))]);
        f.top()
            .env
            .borrow_mut()
            .define(&SymbolId::from("x"), Val::string("hi"));

        start(&mut f).unwrap();
        assert_eq!(f.status, Status::Completed(Ok(Val::string("hi"))));
    }

    #[test]
    #[traced_test]
    fn get_symbol_undefined() {
        let mut f = Fiber::from_bytecode(vec![GetSym(SymbolId::from("x"))]);

        start(&mut f).unwrap();
        assert_matches!(f.status, Status::Completed(Err(Error::UndefinedSymbol(_))));

        assert!(!logs_contain("ERROR"));
        assert!(!logs_contain("WARN"));
    }

    #[test]
    #[traced_test]
    fn set_symbol() {
        let mut f = Fiber::from_bytecode(vec![
            PushConst(Val::string("updated")),
            SetSym(SymbolId::from("x")),
        ]);
        f.top()
            .env
            .borrow_mut()
            .define(&SymbolId::from("x"), Val::string("original"));

        start(&mut f).unwrap();
        assert_eq!(f.status, Status::Completed(Ok(Val::string("updated"))));
    }

    #[test]
    #[traced_test]
    fn set_symbol_undefined() {
        let mut f = Fiber::from_bytecode(vec![
            PushConst(Val::string("value")),
            SetSym(SymbolId::from("x")),
        ]);

        start(&mut f).unwrap();
        assert_matches!(f.status, Status::Completed(Err(Error::UndefinedSymbol(_))));

        assert!(!logs_contain("ERROR"));
        assert!(!logs_contain("WARN"));
    }

    #[test]
    #[traced_test]
    fn make_func() {
        let mut f = Fiber::from_bytecode(vec![
            PushConst(Val::List(vec![Val::symbol("x")])),
            PushConst(Val::Bytecode(vec![GetSym(SymbolId::from("x"))])),
            MakeFunc,
        ]);

        start(&mut f).unwrap();

        assert_matches!(
            f.status,
            Status::Completed(Ok(Val::Lambda(l))) if l.params == vec![SymbolId::from("x")] && l.code == vec![GetSym(SymbolId::from("x"))],
            "A function object was created"
        );

        assert!(!logs_contain("ERROR"));
        assert!(!logs_contain("WARN"));
    }

    #[test]
    #[traced_test]
    fn call_func() {
        let env = Env::default();
        let mut f = Fiber::from_bytecode(vec![
            PushConst(Val::Lambda(Lambda {
                params: vec![SymbolId::from("x")],
                code: vec![GetSym(SymbolId::from("x"))],
                env: Rc::new(RefCell::new(env)),
            })),
            PushConst(Val::string("hello")),
            CallFunc(1),
        ]);

        start(&mut f).unwrap();
        assert_eq!(f.status, Status::Completed(Ok(Val::string("hello"))));

        assert!(!logs_contain("ERROR"));
        assert!(!logs_contain("WARN"));
    }

    #[test]
    #[traced_test]
    fn call_func_lambda() {
        // ((lambda (x) x) "hello")
        let mut f = Fiber::from_bytecode(vec![
            PushConst(Val::List(vec![Val::symbol("x")])),
            PushConst(Val::Bytecode(vec![GetSym(SymbolId::from("x"))])),
            MakeFunc,
            PushConst(Val::string("hello")),
            CallFunc(1),
        ]);
        start(&mut f).unwrap();
        assert_eq!(f.status, Status::Completed(Ok(Val::string("hello"))));

        assert!(!logs_contain("ERROR"));
        assert!(!logs_contain("WARN"));
    }

    #[test]
    #[traced_test]
    fn call_func_nested() {
        // (((lambda () (lambda (x) x))) "hello")
        let mut f = Fiber::from_bytecode(vec![
            PushConst(Val::List(vec![])),
            PushConst(Val::Bytecode(vec![
                PushConst(Val::List(vec![Val::symbol("x")])),
                PushConst(Val::Bytecode(vec![GetSym(SymbolId::from("x"))])),
                MakeFunc,
            ])),
            MakeFunc,
            CallFunc(0),
            PushConst(Val::string("hello")),
            CallFunc(1),
        ]);
        start(&mut f).unwrap();
        assert_eq!(f.status, Status::Completed(Ok(Val::string("hello"))));

        // (((lambda (x) (lambda () x)) "hello"))
        let mut f = Fiber::from_bytecode(vec![
            PushConst(Val::List(vec![Val::symbol("x")])),
            PushConst(Val::Bytecode(vec![
                PushConst(Val::List(vec![])),
                PushConst(Val::Bytecode(vec![GetSym(SymbolId::from("x"))])),
                MakeFunc,
            ])),
            MakeFunc,
            PushConst(Val::string("hello")),
            CallFunc(1),
            CallFunc(0),
        ]);
        start(&mut f).unwrap();
        assert_eq!(f.status, Status::Completed(Ok(Val::string("hello"))));

        assert!(!logs_contain("ERROR"));
        assert!(!logs_contain("WARN"));
    }

    #[test]
    #[traced_test]
    fn pop_top() {
        let mut f = Fiber::from_bytecode(vec![
            PushConst(Val::string("this")),
            PushConst(Val::string("not this")),
            PopTop,
        ]);

        start(&mut f).unwrap();

        assert_eq!(f.status, Status::Completed(Ok(Val::string("this"))));

        assert!(!logs_contain("ERROR"));
        assert!(!logs_contain("WARN"));
    }

    #[test]
    #[traced_test]
    fn pop_top_empty() {
        let mut f = Fiber::from_bytecode(vec![PopTop]);

        start(&mut f).unwrap();

        assert_matches!(f.status, Status::Completed(Err(Error::UnexpectedStack(_))));

        assert!(!logs_contain("ERROR"));
        assert!(!logs_contain("WARN"));
    }
}
