//! A fiber of execution that can be cooperatively scheduled via yielding.
use tracing::{debug, warn};

use super::{Env, Inst};
use crate::{compile, parse, Error, Lambda, NativeFn, NativeFnVal, Result, Val};
use std::{cell::RefCell, rc::Rc};

/// A single, cooperativly scheduled sequence of execution
#[derive(Debug)]
pub struct Fiber {
    cframes: Vec<CallFrame>,
    stack: Vec<Val>,
    global: Rc<RefCell<Env>>,
    is_yielding: bool,
}

/// Result from fiber execution
#[derive(Debug, PartialEq)]
pub enum FiberState {
    Yield(Val),
    Done(Val),
}

impl Fiber {
    /// Create a new fiber from given bytecode
    pub fn from_bytecode(bytecode: Vec<Inst>) -> Self {
        let global = Rc::new(RefCell::new(Env::standard()));
        Fiber {
            stack: vec![],
            cframes: vec![CallFrame::from_bytecode(Rc::clone(&global), bytecode)],
            global,
            is_yielding: false,
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

    /// Set root environment of fiber
    pub fn with_env(mut self, env: Rc<RefCell<Env>>) -> Self {
        // TODO: This is a hack for peval. Replace with builder for setting bytecode + env for new processes
        assert_eq!(
            self.cframes.len(),
            1,
            "hack to override root callstack is only avaiable to fibers at root callstack 1"
        );
        self.global = Rc::clone(&env);
        self.top_mut().env = env;
        self
    }

    /// Start execution of a fiber
    pub fn resume(&mut self) -> Result<FiberState> {
        // TODO: Better safeguards for resume vs resume_from_yield
        if self.is_yielding {
            return Err(Error::UnexpectedResume(
                "resuming a yielding fiber without value".to_string(),
            ));
        }
        run(self)
    }

    /// Resume a yielded fiber
    pub fn resume_from_yield(&mut self, v: Val) -> Result<FiberState> {
        if !self.is_yielding {
            return Err(Error::UnexpectedResume(
                "resuming a nonyielding fiber".to_string(),
            ));
        }
        self.is_yielding = false;
        self.stack.push(v);
        run(self)
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

    /// Get current stack's environment
    pub fn env(&self) -> Rc<RefCell<Env>> {
        Rc::clone(&self.top().env)
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
    fn inst(&self) -> Result<&Inst> {
        let top = self.top();
        let inst = top.code.get(top.ip).ok_or(Error::NoMoreBytecode)?;
        Ok(inst)
    }

    /// Exhausted all call frames
    fn at_end(&self) -> bool {
        self.cframes.len() == 1 && self.cframes.last().unwrap().is_done()
    }
}

/// Single call frame of fiber
#[derive(Debug)]
struct CallFrame {
    ip: usize,
    code: Vec<Inst>,
    env: Rc<RefCell<Env>>,
}

impl CallFrame {
    /// Create a new callframe for executing given bytecode from start
    fn from_bytecode(env: Rc<RefCell<Env>>, code: Vec<Inst>) -> Self {
        Self { ip: 0, env, code }
    }

    /// Whether or not call frame should return (i.e. on last call frame execution)
    fn is_done(&self) -> bool {
        self.ip == self.code.len()
    }
}

// TODO: Refactor - Fiber Internals for `run`
/// Run the fiber until it completes or yields
fn run(f: &mut Fiber) -> Result<FiberState> {
    loop {
        if f.is_yielding {
            let res = f.stack.pop().ok_or(Error::UnexpectedStack(
                "Stack should contain result for terminated fiber".to_string(),
            ))?;
            return Ok(FiberState::Yield(res));
        } else if f.at_end() {
            let res = f.stack.pop().ok_or(Error::UnexpectedStack(
                "Stack should contain result for terminated fiber".to_string(),
            ))?;
            if !f.stack.is_empty() {
                warn!("Fiber terminated with nonempty stack {:?}", f.stack);
            }
            return Ok(FiberState::Done(res));
        }

        let inst = f.inst()?.clone(); // TODO(opt): Defer cloning until args need cloning
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
            Inst::DefSym(s) => {
                let value = f.stack.last().ok_or(Error::UnexpectedStack(
                    "Expected stack to be nonempty".to_string(),
                ))?;
                f.top().env.borrow_mut().define(&s, value.clone());
            }
            Inst::SetSym(s) => {
                let value = f.stack.last().ok_or(Error::UnexpectedStack(
                    "Expected stack to be nonempty".to_string(),
                ))?;
                f.top().env.borrow_mut().set(&s, value.clone())?
            }
            Inst::GetSym(s) => {
                let value = f
                    .top()
                    .env
                    .clone()
                    .borrow()
                    .get(&s)
                    .ok_or(Error::UndefinedSymbol(s))?;
                f.stack.push(value.clone());
            }
            Inst::MakeFunc => {
                let code = match f.stack.pop() {
                    Some(Val::Bytecode(b)) => Ok(b),
                    _ => Err(Error::UnexpectedStack(
                        "Missing function bytecode".to_string(),
                    )),
                }?;

                let params = match f.stack.pop() {
                    Some(Val::List(p)) => Ok(p),
                    _ => Err(Error::UnexpectedStack("Missing parameter list".to_string())),
                }?;

                let params = params
                    .into_iter()
                    .map(|f| match f {
                        Val::Symbol(s) => Ok(s),
                        _ => Err(Error::UnexpectedStack(
                            "Unexpected parameter list".to_string(),
                        )),
                    })
                    .collect::<Result<Vec<_>>>()?;

                f.stack.push(Val::Lambda(Lambda {
                    params,
                    code,
                    env: Rc::clone(&f.top().env),
                }));
            }
            Inst::CallFunc(nargs) => {
                let mut args = vec![];
                for _ in 0..nargs {
                    let v = f.stack.pop().ok_or(Error::UnexpectedStack(
                        "Missing expected {nargs} args".to_string(),
                    ))?;
                    args.push(v);
                }
                let args = args.into_iter().rev();

                match f.stack.pop() {
                    Some(Val::NativeFn(n)) => {
                        let v = (n.func)(f, &args.collect::<Vec<_>>())?;
                        match v {
                            NativeFnVal::Return(v) => f.stack.push(v),
                            NativeFnVal::Yield(v) => {
                                f.stack.push(v);
                                f.is_yielding = true;
                            }
                        }
                    }
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
                        return Err(Error::UnexpectedStack(
                            "Missing function object".to_string(),
                        ))
                    }
                };
            }
            Inst::Eval => {
                let val = f.stack.pop().ok_or(Error::UnexpectedStack(
                    "Did not find form to eval on stack".to_string(),
                ))?;
                let bc = compile(&val)?;
                let cur_env = &f.top().env;
                f.cframes
                    .push(CallFrame::from_bytecode(Rc::clone(cur_env), bc));
            }
            Inst::PopTop => {
                if f.stack.pop().is_none() {
                    return Err(Error::UnexpectedStack(
                        "Attempting to pop empty stack".to_string(),
                    ));
                }
            }
            Inst::JumpFwd(fwd) => f.top_mut().ip += fwd,
            Inst::PopJumpFwdIfTrue(offset) => {
                let v = f.stack.pop().ok_or(Error::UnexpectedStack(
                    "Expected conditional expression on stack".to_string(),
                ))?;
                if is_true(v)? {
                    f.top_mut().ip += offset;
                }
            }
            Inst::JumpBck(back) => f.top_mut().ip -= back,
            Inst::YieldTop => f.is_yielding = true,
        }

        // Implicit returns - Pop completed frames except root
        while f.cframes.len() > 1 && f.top().is_done() {
            f.cframes.pop();
        }
    }
}

/// Defines true values
fn is_true(v: Val) -> Result<bool> {
    let cond = match v {
        Val::Nil => false,
        Val::Bool(b) => b,
        Val::Int(i) => i != 0,
        Val::String(s) => !s.is_empty(),
        Val::List(l) => !l.is_empty(),
        v => {
            return Err(Error::UnexpectedArguments(format!(
                "Value is not a valid condition - {v}"
            )))
        }
    };
    Ok(cond)
}

#[cfg(test)]
mod tests {
    use super::FiberState::*;
    use super::Inst::*;
    use super::*;
    use crate::SymbolId;
    use assert_matches::assert_matches;
    use tracing_test::traced_test;

    #[test]
    #[traced_test]
    fn fiber_load_const_return() {
        let mut f = Fiber::from_bytecode(vec![PushConst(Val::Int(5))]);
        assert_eq!(f.resume().unwrap(), Done(Val::Int(5)));

        let mut f = Fiber::from_bytecode(vec![PushConst(Val::string("Hi"))]);
        assert_eq!(f.resume().unwrap(), Done(Val::string("Hi")));

        assert!(!logs_contain("ERROR"));
        assert!(!logs_contain("WARN"));
    }

    #[test]
    #[traced_test]
    fn def_symbol() {
        // fiber for (def x 5)
        let mut f = Fiber::from_bytecode(vec![PushConst(Val::Int(5)), DefSym(SymbolId::from("x"))]);
        assert_eq!(f.resume().unwrap(), Done(Val::Int(5)));
        assert_eq!(
            f.top().env.borrow().get(&SymbolId::from("x")),
            Some(Val::Int(5)),
            "symbol should be defined in env"
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
        assert_eq!(f.resume().unwrap(), Done(Val::string("hi")));
    }

    #[test]
    #[traced_test]
    fn get_symbol_undefined() {
        let mut f = Fiber::from_bytecode(vec![GetSym(SymbolId::from("x"))]);
        assert_matches!(f.resume(), Err(Error::UndefinedSymbol(_)));
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

        assert_eq!(f.resume().unwrap(), Done(Val::string("updated")));
    }

    #[test]
    #[traced_test]
    fn set_symbol_undefined() {
        let mut f = Fiber::from_bytecode(vec![
            PushConst(Val::string("value")),
            SetSym(SymbolId::from("x")),
        ]);

        assert_matches!(f.resume(), Err(Error::UndefinedSymbol(_)));

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

        assert_matches!(
            f.resume().unwrap(),
            Done(Val::Lambda(l)) if l.params == vec![SymbolId::from("x")] && l.code == vec![GetSym(SymbolId::from("x"))],
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

        assert_eq!(f.resume().unwrap(), Done(Val::string("hello")));

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

        assert_eq!(f.resume().unwrap(), Done(Val::string("hello")));

        assert!(!logs_contain("ERROR"));
        assert!(!logs_contain("WARN"));
    }

    #[test]
    #[traced_test]
    fn call_func_nested() {
        // Call outer lambda w/ arg
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
        assert_eq!(f.resume().unwrap(), Done(Val::string("hello")));

        // Call inner lambda w/ arg
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
        assert_eq!(f.resume().unwrap(), Done(Val::string("hello")));

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
        assert_eq!(f.resume().unwrap(), Done(Val::string("this")));

        assert!(!logs_contain("ERROR"));
        assert!(!logs_contain("WARN"));
    }

    #[test]
    #[traced_test]
    fn pop_top_empty() {
        let mut f = Fiber::from_bytecode(vec![PopTop]);
        assert_matches!(f.resume(), Err(Error::UnexpectedStack(_)));

        assert!(!logs_contain("ERROR"));
        assert!(!logs_contain("WARN"));
    }

    #[test]
    #[traced_test]
    fn jump_fwd() {
        let mut f = Fiber::from_bytecode(vec![
            PushConst(Val::string("this")),
            JumpFwd(5),
            PushConst(Val::string("notthis")),
            PushConst(Val::string("notthis")),
            PushConst(Val::string("notthis")),
            PushConst(Val::string("notthis")),
            PushConst(Val::string("notthis")),
        ]);

        assert_eq!(f.resume().unwrap(), Done(Val::string("this")));

        assert!(!logs_contain("ERROR"));
        assert!(!logs_contain("WARN"));
    }

    #[test]
    #[traced_test]
    fn pop_jump_fwd_true() {
        let mut f = Fiber::from_bytecode(vec![
            PushConst(Val::Bool(true)),
            PopJumpFwdIfTrue(2),
            PushConst(Val::string("notthis")),
            JumpFwd(1),
            PushConst(Val::string("this")),
        ]);

        assert_eq!(f.resume().unwrap(), Done(Val::string("this")));

        assert!(!logs_contain("ERROR"));
        assert!(!logs_contain("WARN"));
    }

    #[test]
    #[traced_test]
    fn pop_jump_fwd_false() {
        let mut f = Fiber::from_bytecode(vec![
            PushConst(Val::Bool(false)),
            PopJumpFwdIfTrue(2),
            PushConst(Val::string("this")),
            JumpFwd(1),
            PushConst(Val::string("notthis")),
        ]);

        assert_eq!(f.resume().unwrap(), Done(Val::string("this")));

        assert!(!logs_contain("ERROR"));
        assert!(!logs_contain("WARN"));
    }

    #[test]
    #[traced_test]
    fn jump_back() {
        let mut f = Fiber::from_bytecode(vec![
            PushConst(Val::Int(0)),
            DefSym(SymbolId::from("x")),
            PopTop,
            GetSym(SymbolId::from("x")),
            PopJumpFwdIfTrue(4), // termination
            PushConst(Val::Int(1)),
            SetSym(SymbolId::from("x")),
            PopTop,
            JumpBck(6), // loop back to getsym
            GetSym(SymbolId::from("x")),
        ]);

        assert_eq!(f.resume().unwrap(), Done(Val::Int(1)));
    }

    #[test]
    #[traced_test]
    fn yield_once() {
        let mut f = Fiber::from_bytecode(vec![PushConst(Val::string("before")), YieldTop]);
        assert_eq!(f.resume().unwrap(), Yield(Val::string("before")));
        assert_eq!(
            f.resume_from_yield(Val::string("after")).unwrap(),
            Done(Val::string("after"))
        );
    }

    #[test]
    #[traced_test]
    fn yield_infinite() {
        let mut f = Fiber::from_bytecode(vec![
            PushConst(Val::Int(0)),
            DefSym(SymbolId::from("x")),
            PopTop,
            GetSym(SymbolId::from("+")),
            GetSym(SymbolId::from("x")),
            PushConst(Val::Int(1)),
            CallFunc(2),
            SetSym(SymbolId::from("x")),
            YieldTop,
            PopTop,
            JumpBck(8),
        ]);

        assert_eq!(f.resume().unwrap(), Yield(Val::Int(1)));
        assert_eq!(f.resume_from_yield(Val::Nil).unwrap(), Yield(Val::Int(2)));
        assert_eq!(f.resume_from_yield(Val::Nil).unwrap(), Yield(Val::Int(3)));
        assert_eq!(f.resume_from_yield(Val::Nil).unwrap(), Yield(Val::Int(4)));
        assert_eq!(f.resume_from_yield(Val::Nil).unwrap(), Yield(Val::Int(5)));
    }

    #[test]
    #[traced_test]
    fn eval() {
        let mut f = Fiber::from_bytecode(vec![PushConst(Val::Int(42)), Eval]);
        assert_eq!(f.resume().unwrap(), Done(Val::Int(42)));

        let mut f = Fiber::from_bytecode(vec![
            PushConst(Val::List(vec![
                Val::symbol("quote"),
                Val::List(vec![
                    Val::keyword("a"),
                    Val::keyword("b"),
                    Val::keyword("c"),
                ]),
            ])),
            Eval,
        ]);

        assert_eq!(
            f.resume().unwrap(),
            Done(Val::List(vec![
                Val::keyword("a"),
                Val::keyword("b"),
                Val::keyword("c"),
            ]))
        );
    }

    #[test]
    #[traced_test]
    fn eval_error() {
        let mut f = Fiber::from_bytecode(vec![PushConst(Val::symbol("jibberish")), Eval]);
        assert_matches!(f.resume(), Err(Error::UndefinedSymbol(_)));

        let mut f = Fiber::from_bytecode(vec![
            PushConst(Val::List(vec![
                Val::symbol("nonexisting"),
                Val::keyword("a"),
                Val::keyword("b"),
                Val::keyword("c"),
            ])),
            Eval,
        ]);
        assert_matches!(f.resume(), Err(Error::UndefinedSymbol(_)));
    }

    #[test]
    #[traced_test]
    fn yield_loop() {
        let mut f = Fiber::from_bytecode(vec![
            PushConst(Val::string("hi")),
            YieldTop,
            PopTop,
            JumpBck(4),
        ]);

        assert_eq!(f.resume().unwrap(), Yield(Val::string("hi")));
        assert!(f.is_stack_empty());

        assert_eq!(
            f.resume_from_yield(Val::Nil).unwrap(),
            Yield(Val::string("hi"))
        );
        assert_eq!(
            f.resume_from_yield(Val::Nil).unwrap(),
            Yield(Val::string("hi"))
        );
        assert_eq!(
            f.resume_from_yield(Val::Nil).unwrap(),
            Yield(Val::string("hi"))
        );
        assert_eq!(
            f.resume_from_yield(Val::Nil).unwrap(),
            Yield(Val::string("hi"))
        );

        assert!(f.is_stack_empty());
    }
}
