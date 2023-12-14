//! A fiber of execution that can be driven by caller as a coroutine.

use super::{Env, Inst};
use crate::{
    builtin::cond::is_true, compile, parse, Bytecode, Error, Extern, Lambda, Locals, NativeFnOp,
    Pattern, Result, Val,
};
use std::sync::{Arc, Mutex};
use tracing::warn;

/// The state of fiber
#[derive(Debug, PartialEq)]
pub enum FiberState {
    /// Fiber was created, and can be started.
    New,
    /// Fiber is paused, and can be resumed
    Paused,
    /// Fiber is currently running
    Running,
    /// Fiber has completeed execution, and cannot be resumed
    Done,
}

/// A single, cooperativly scheduled sequence of execution with its own stack
/// and environment.  The fiber can be a value in Lyric environment, and
/// interacted as a coroutine to implement generators, etc.
///
/// Fibers executing at the root-level may rely on convenience primitives like
/// [lyric::Process] to drive execution forward
#[derive(Debug)]
pub struct Fiber<T: Extern, L: Locals> {
    state: FiberState,
    cframes: Vec<CallFrame<T, L>>,
    stack: Vec<Val<T, L>>,
    global: Arc<Mutex<Env<T, L>>>,
    locals: L,
}

/// Single call frame of fiber
#[derive(Debug)]
struct CallFrame<T: Extern, L: Locals> {
    /// instruction pointer in code
    ip: usize,
    /// Code in callframe
    code: Bytecode<T, L>,
    /// Environment this callframe is operating in
    env: Arc<Mutex<Env<T, L>>>,
    /// Length of stack when callframe was created
    stack_len: usize,
    /// Length of callframe of fiber to unwind to on error, if any
    unwind_cf_len: Option<usize>,
}

impl<T: Extern, L: Locals> Fiber<T, L> {
    /// Create a new fiber from given bytecode
    pub fn from_bytecode(bytecode: Bytecode<T, L>, env: Env<T, L>, locals: L) -> Self {
        let global = Arc::new(Mutex::new(env));
        Fiber {
            state: FiberState::New,
            stack: vec![],
            cframes: vec![CallFrame::from_bytecode(
                Arc::clone(&global),
                bytecode,
                0,
                None,
            )],
            global,
            locals,
        }
    }

    /// Create a new fiber from value
    pub fn from_val(val: &Val<T, L>, env: Env<T, L>, locals: L) -> Result<Self> {
        let bytecode = compile(val)?;
        Ok(Fiber::from_bytecode(bytecode, env, locals))
    }

    /// Create a new fiber from given expressino
    pub fn from_expr(expr: &str, env: Env<T, L>, locals: L) -> Result<Self> {
        let val: Val<T, L> = parse(expr)?.into();
        Fiber::from_val(&val, env, locals)
    }

    // TODO: Safeguard start / resume via typestate pat?

    /// Start a fiber execution
    pub fn start(&mut self) -> Result<Val<T, L>> {
        if self.state != FiberState::New {
            return Err(Error::UnexpectedResume(
                "starting a fiber that is not new".to_string(),
            ));
        }
        self.run()
    }

    /// Resume a paused fiber execution
    pub fn resume(&mut self, val: Val<T, L>) -> Result<Val<T, L>> {
        if self.state != FiberState::Paused {
            return Err(Error::UnexpectedResume(
                "resuming a fiber that is not paused".to_string(),
            ));
        }
        self.stack.push(val);
        self.run()
    }

    /// Whether or not fiber is done running
    pub fn is_done(&self) -> bool {
        self.state == FiberState::Done
    }

    /// Get current environment
    pub fn cur_env(&self) -> &Arc<Mutex<Env<T, L>>> {
        &self.cf().env
    }

    /// Get the global environment
    pub fn global_env(&self) -> &Arc<Mutex<Env<T, L>>> {
        &self.global
    }

    /// Local storage
    pub fn locals(&self) -> &L {
        &self.locals
    }

    /// Mutable Local storage
    pub fn locals_mut(&mut self) -> &mut L {
        &mut self.locals
    }
}

impl<T: Extern, L: Locals> Fiber<T, L> {
    /// Run a fiber execution until it:
    /// - completes with a value
    /// - completes with an error
    /// - becomes paused
    fn run(&mut self) -> Result<Val<T, L>> {
        self.state = FiberState::Running;
        while self.state == FiberState::Running {
            // TODO(dev): Bytecode debugging utilities
            // tracing::debug!("{self:?}");

            if let Err(e) = self.step() {
                // Catch unwind or exit w/ error
                let unwind_len = match self.cf().unwind_cf_len {
                    None => {
                        self.state = FiberState::Done;
                        return Err(e);
                    }
                    Some(l) => l,
                };
                let stack_len = self.cframes[unwind_len].stack_len;
                self.cframes.truncate(unwind_len);
                self.stack.truncate(stack_len);
                self.stack.push(Val::Error(e));
            }

            // Cleanup for implicit returns in call frames
            // Must be after unwinding or catching context is lost
            while self.cframes.len() > 1 && self.cf().at_return() {
                let cf = self.cframes.pop().unwrap();
                if self.stack.len() != cf.stack_len + 1 {
                    panic!("Unexpected state during execution - all function are expected to have stack effect of 1");
                }
            }
        }

        match &self.state {
            FiberState::Paused => {
                let res = self.stack.pop().ok_or(Error::UnexpectedStack(
                    "Stack should contain result for terminated fiber".to_string(),
                ))?;
                Ok(res)
            }
            FiberState::Done => {
                let res = self.stack.pop().ok_or(Error::UnexpectedStack(
                    "Stack should contain result for terminated fiber".to_string(),
                ))?;
                if !self.stack.is_empty() {
                    warn!("Fiber terminated with nonempty stack {:?}", self.stack);
                }
                Ok(res)
            }
            s => panic!("Fiber::run exiting in unexpected state - {s:?}"),
        }
    }

    /// Run a single fetch-decode-execute cycle
    fn step(&mut self) -> Result<()> {
        let inst = match self.inst() {
            Some(i) => i.clone(),
            None => {
                self.state = FiberState::Done;
                return Ok(());
            }
        };
        self.cf_mut().ip += 1;

        match inst {
            Inst::PushConst(form) => {
                self.stack.push(form);
            }
            Inst::DefSym(s) => {
                let value = self.stack.last().ok_or(Error::UnexpectedStack(
                    "Stack should contain value to bind".to_string(),
                ))?;
                self.cur_env()
                    .lock()
                    .unwrap()
                    .define(s.clone(), value.clone());
            }
            Inst::DefBind => {
                let pat = self.stack.pop().ok_or(Error::UnexpectedStack(
                    "Stack should contain pattern to bind to".to_string(),
                ))?;
                let val = self.stack.last().ok_or(Error::UnexpectedStack(
                    "Stack should contain value to bind".to_string(),
                ))?;

                let m = Pattern::from_val(pat)
                    .matches(val)
                    .ok_or(Error::InvalidPatternMatch)?;

                let mut env = self.cur_env().lock().unwrap();
                for (s, v) in m.into_iter() {
                    env.define(s, v.clone());
                }
            }
            Inst::SetSym(s) => {
                let value = self.stack.last().ok_or(Error::UnexpectedStack(
                    "Stack should contain value to bind".to_string(),
                ))?;
                self.cur_env().lock().unwrap().set(&s, value.clone())?
            }
            Inst::GetSym(s) => {
                let value = self
                    .cur_env()
                    .clone()
                    .lock()
                    .unwrap()
                    .get(&s)
                    .ok_or(Error::UndefinedSymbol(s))?;
                self.stack.push(value.clone());
            }
            Inst::MakeFunc => {
                let code = match self.stack.pop() {
                    Some(Val::Bytecode(b)) => Ok(b),
                    _ => Err(Error::UnexpectedStack(
                        "Missing function bytecode".to_string(),
                    )),
                }?;
                let params = match self.stack.pop() {
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

                self.stack.push(Val::Lambda(Lambda {
                    params,
                    code,
                    parent: Some(Arc::clone(&self.cf().env)),
                }));
            }
            Inst::CallFunc(nargs) => {
                let mut args = vec![];
                for _ in 0..nargs {
                    let v = self.stack.pop().ok_or(Error::UnexpectedStack(
                        "Missing expected {nargs} args".to_string(),
                    ))?;
                    args.push(v);
                }
                let args = args.into_iter().rev();

                match self.stack.pop() {
                    Some(Val::Lambda(l)) => {
                        let parent_env = l.parent.unwrap_or_else(|| Arc::clone(&self.global));
                        let mut fn_env = Env::extend(&parent_env);
                        for (s, arg) in l.params.into_iter().zip(args) {
                            fn_env.define(s, arg);
                        }
                        self.cframes.push(CallFrame::from_bytecode(
                            Arc::new(Mutex::new(fn_env)),
                            l.code,
                            self.stack.len(),
                            self.cf().unwind_cf_len,
                        ))
                    }
                    Some(Val::NativeFn(n)) => {
                        let v = (n.func)(self, &args.collect::<Vec<_>>())?;
                        match v {
                            NativeFnOp::Return(v) => self.stack.push(v),
                            NativeFnOp::Yield(v) => {
                                self.stack.push(v);
                                self.state = FiberState::Paused;
                            }
                            NativeFnOp::Exec(code) => self.cframes.push(CallFrame::from_bytecode(
                                Arc::clone(self.cur_env()),
                                code,
                                self.stack.len(),
                                self.cf().unwind_cf_len,
                            )),
                        }
                    }
                    // TODO: Move into NativeFnAsync (?) Remove Yield keyword out of NativeFs
                    Some(obj) => {
                        return Err(Error::UnexpectedStack(format!(
                            "Not a function object - {}",
                            obj
                        )));
                    }
                    _ => {
                        return Err(Error::UnexpectedStack("Stack is empty".to_string()));
                    }
                };
            }
            Inst::Eval(protected) => {
                // set new protected frame or inherit
                let unwind_cf_len = match protected {
                    true => Some(self.cframes.len()),
                    false => self.cf().unwind_cf_len,
                };
                let val = self.stack.pop().ok_or(Error::UnexpectedStack(
                    "Did not find form to eval on stack".to_string(),
                ))?;
                let bc = compile(&val)?;
                self.cframes.push(CallFrame::from_bytecode(
                    Arc::clone(self.cur_env()),
                    bc,
                    self.stack.len(),
                    unwind_cf_len,
                ));
            }
            Inst::PopTop => {
                if self.stack.pop().is_none() {
                    return Err(Error::UnexpectedStack(
                        "Attempting to pop empty stack".to_string(),
                    ));
                }
            }
            Inst::JumpFwd(fwd) => self.cf_mut().ip += fwd,
            Inst::JumpBck(back) => self.cf_mut().ip -= back,
            Inst::PopJumpFwdIfTrue(offset) => {
                let v = self.stack.pop().ok_or(Error::UnexpectedStack(
                    "Expected conditional expression on stack".to_string(),
                ))?;
                if is_true(&v)? {
                    self.cf_mut().ip += offset;
                }
            }
            Inst::YieldTop => self.state = FiberState::Paused,
        };

        Ok(())
    }

    /// Next instruction in fiber, or None if fiber is complete
    fn inst(&self) -> Option<&Inst<T, L>> {
        let cf = self.cf();
        cf.code.get(cf.ip)
    }

    /// Top callframe
    fn cf(&self) -> &CallFrame<T, L> {
        self.cframes.last().expect("Fiber has no callframes!")
    }

    /// Mutable top callframe
    fn cf_mut(&mut self) -> &mut CallFrame<T, L> {
        self.cframes.last_mut().expect("Fiber has no callframes!")
    }
}

impl<T: Extern, L: Locals> CallFrame<T, L> {
    /// Create a new callframe for executing given bytecode from start
    fn from_bytecode(
        env: Arc<Mutex<Env<T, L>>>,
        code: Bytecode<T, L>,
        stack_len: usize,
        unwind_cf_len: Option<usize>,
    ) -> Self {
        Self {
            ip: 0,
            env,
            code,
            stack_len,
            unwind_cf_len,
        }
    }

    /// Whether or not call frame is at implicit return
    fn at_return(&self) -> bool {
        self.ip == self.code.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_matches::assert_matches;
    use void::Void;

    use super::Inst::*;
    use crate::SymbolId;

    type Fiber = super::Fiber<Void, ()>;
    type Val = super::Val<Void, ()>;
    type Env = super::Env<Void, ()>;

    #[test]
    fn empty() {
        let mut f = Fiber::from_bytecode(vec![], Env::standard(), ());

        assert!(!f.is_done());
        assert_matches!(f.start(), Err(Error::UnexpectedStack(_)));
        assert!(f.is_done());
    }

    #[test]
    fn done_with_result() {
        let mut f = Fiber::from_bytecode(vec![PushConst(Val::Int(5))], Env::standard(), ());

        assert!(!f.is_done());
        assert!(f.start().is_ok());
        assert!(f.is_done());

        assert_matches!(
            f.start(),
            Err(Error::UnexpectedResume(_)),
            "Should not be able to start after result"
        );
        assert_matches!(
            f.resume(Val::Nil),
            Err(Error::UnexpectedResume(_)),
            "Should not be able to resume after result"
        );
    }

    #[test]
    fn done_with_error() {
        let mut f = Fiber::from_bytecode(
            vec![
                GetSym(SymbolId::from("x")),
                // rest should be ignored after err
                PopTop,
                PopTop,
                PopTop,
            ],
            Env::standard(),
            (),
        );

        assert!(f.start().is_err());
        assert!(f.is_done());

        assert_matches!(
            f.start(),
            Err(Error::UnexpectedResume(_)),
            "Should not be able to start after error"
        );
        assert_matches!(
            f.resume(Val::Nil),
            Err(Error::UnexpectedResume(_)),
            "Should not be able to resume after error"
        );
    }

    #[test]
    fn push_const() {
        {
            let mut f = Fiber::from_bytecode(vec![PushConst(Val::Int(5))], Env::standard(), ());
            assert!(!f.is_done());
            assert_eq!(f.start().unwrap(), Val::Int(5));
            assert!(f.is_done());
        }

        {
            let mut f =
                Fiber::from_bytecode(vec![PushConst(Val::string("Hi"))], Env::standard(), ());
            assert_eq!(f.start().unwrap(), Val::string("Hi"));
        }
    }

    #[test]
    fn def_symbol() {
        let mut f = Fiber::from_bytecode(
            vec![PushConst(Val::Int(5)), DefSym(SymbolId::from("x"))],
            Env::standard(),
            (),
        );

        assert_eq!(f.start().unwrap(), Val::Int(5));
        assert_eq!(
            f.cf().env.lock().unwrap().get(&SymbolId::from("x")),
            Some(Val::Int(5)),
            "symbol should be defined in env"
        );
    }

    #[test]
    fn get_symbol() {
        let mut f = Fiber::from_bytecode(vec![GetSym(SymbolId::from("x"))], Env::standard(), ());
        f.cf()
            .env
            .lock()
            .unwrap()
            .define(SymbolId::from("x"), Val::string("hi"));
        assert_eq!(f.start().unwrap(), Val::string("hi"));
        assert!(f.is_done());
    }

    #[test]
    fn get_symbol_undefined() {
        let mut f = Fiber::from_bytecode(vec![GetSym(SymbolId::from("x"))], Env::standard(), ());

        assert_matches!(f.start(), Err(Error::UndefinedSymbol(_)));
        assert!(f.is_done(), "Should be done after error");
    }

    #[test]
    fn set_symbol() {
        let mut f = Fiber::from_bytecode(
            vec![
                PushConst(Val::string("updated")),
                SetSym(SymbolId::from("x")),
            ],
            Env::standard(),
            (),
        );
        f.cf()
            .env
            .lock()
            .unwrap()
            .define(SymbolId::from("x"), Val::string("original"));

        assert_eq!(f.start().unwrap(), Val::string("updated"));
        assert!(f.is_done());
    }

    #[test]
    fn set_symbol_undefined() {
        let mut f = Fiber::from_bytecode(
            vec![PushConst(Val::string("value")), SetSym(SymbolId::from("x"))],
            Env::standard(),
            (),
        );

        assert_matches!(f.start(), Err(Error::UndefinedSymbol(_)));
        assert!(f.is_done());
    }

    #[test]
    fn make_func() {
        let mut f = Fiber::from_bytecode(
            vec![
                PushConst(Val::List(vec![Val::symbol("x")])),
                PushConst(Val::Bytecode(vec![GetSym(SymbolId::from("x"))])),
                MakeFunc,
            ],
            Env::standard(),
            (),
        );

        assert_matches!(
            f.start().unwrap(),
            Val::Lambda(l) if l.params == vec![SymbolId::from("x")] && l.code == vec![GetSym(SymbolId::from("x"))],
            "A function object was created"
        );
        assert!(f.is_done());
    }

    #[test]
    fn call_func() {
        let mut f = Fiber::from_bytecode(
            vec![
                PushConst(Val::Lambda(Lambda {
                    params: vec![SymbolId::from("x")],
                    code: vec![GetSym(SymbolId::from("x"))],
                    parent: None,
                })),
                PushConst(Val::string("hello")),
                CallFunc(1),
            ],
            Env::standard(),
            (),
        );

        assert_eq!(f.start().unwrap(), Val::string("hello"));
        assert!(f.is_done());
    }

    #[test]
    fn call_func_lambda() {
        // ((lambda (x) x) "hello")
        let mut f = Fiber::from_bytecode(
            vec![
                PushConst(Val::List(vec![Val::symbol("x")])),
                PushConst(Val::Bytecode(vec![GetSym(SymbolId::from("x"))])),
                MakeFunc,
                PushConst(Val::string("hello")),
                CallFunc(1),
            ],
            Env::standard(),
            (),
        );

        assert_eq!(f.start().unwrap(), Val::string("hello"));
        assert!(f.is_done())
    }

    #[test]
    fn call_func_nested() {
        // Call outer lambda w/ arg
        // (((lambda () (lambda (x) x))) "hello")
        let mut f = Fiber::from_bytecode(
            vec![
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
            ],
            Env::standard(),
            (),
        );
        assert_eq!(f.start().unwrap(), Val::string("hello"));
        assert!(f.is_done());

        // Call inner lambda w/ arg
        // (((lambda (x) (lambda () x)) "hello"))
        let mut f = Fiber::from_bytecode(
            vec![
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
            ],
            Env::standard(),
            (),
        );
        assert_eq!(f.start().unwrap(), Val::string("hello"));
        assert!(f.is_done())
    }

    #[test]
    fn pop_top() {
        let mut f = Fiber::from_bytecode(
            vec![
                PushConst(Val::string("this")),
                PushConst(Val::string("not this")),
                PopTop,
            ],
            Env::standard(),
            (),
        );
        assert_eq!(f.start().unwrap(), Val::string("this"));
        assert!(f.is_done())
    }

    #[test]
    fn pop_top_empty() {
        let mut f = Fiber::from_bytecode(vec![PopTop], Env::standard(), ());
        assert_matches!(f.start(), Err(Error::UnexpectedStack(_)));
    }

    #[test]
    fn jump_fwd() {
        let mut f = Fiber::from_bytecode(
            vec![
                PushConst(Val::string("this")),
                JumpFwd(5),
                PushConst(Val::string("notthis")),
                PushConst(Val::string("notthis")),
                PushConst(Val::string("notthis")),
                PushConst(Val::string("notthis")),
                PushConst(Val::string("notthis")),
            ],
            Env::standard(),
            (),
        );

        assert_eq!(f.start().unwrap(), Val::string("this"));
        assert!(f.is_done())
    }

    #[test]
    fn pop_jump_fwd_true() {
        let mut f = Fiber::from_bytecode(
            vec![
                PushConst(Val::Bool(true)),
                PopJumpFwdIfTrue(2),
                PushConst(Val::string("notthis")),
                JumpFwd(1),
                PushConst(Val::string("this")),
            ],
            Env::standard(),
            (),
        );

        assert_eq!(f.start().unwrap(), Val::string("this"));
        assert!(f.is_done())
    }

    #[test]
    fn pop_jump_fwd_false() {
        let mut f = Fiber::from_bytecode(
            vec![
                PushConst(Val::Bool(false)),
                PopJumpFwdIfTrue(2),
                PushConst(Val::string("this")),
                JumpFwd(1),
                PushConst(Val::string("notthis")),
            ],
            Env::standard(),
            (),
        );

        assert_eq!(f.start().unwrap(), Val::string("this"));
        assert!(f.is_done())
    }

    #[test]
    fn jump_back() {
        let mut f = Fiber::from_bytecode(
            vec![
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
            ],
            Env::standard(),
            (),
        );

        assert_eq!(f.start().unwrap(), Val::Int(1));
        assert!(f.is_done())
    }

    #[test]
    fn yield_once() {
        let mut f = Fiber::from_bytecode(
            vec![PushConst(Val::string("before")), YieldTop],
            Env::standard(),
            (),
        );
        assert_eq!(f.start().unwrap(), Val::string("before"));
        assert!(!f.is_done());
        assert_eq!(
            f.resume(Val::string("after")).unwrap(),
            Val::string("after")
        );
        assert!(f.is_done());
    }

    #[test]
    fn yield_infinite() {
        let mut f = Fiber::from_bytecode(
            vec![
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
            ],
            Env::standard(),
            (),
        );

        assert_eq!(f.start().unwrap(), Val::Int(1));
        assert_eq!(f.resume(Val::Nil).unwrap(), Val::Int(2));
        assert_eq!(f.resume(Val::Nil).unwrap(), Val::Int(3));
        assert_eq!(f.resume(Val::Nil).unwrap(), Val::Int(4));
        assert_eq!(f.resume(Val::Nil).unwrap(), Val::Int(5));

        assert!(!f.is_done(), "infinitely yielding fiber is not done");
    }

    #[test]
    fn eval() {
        let mut f = Fiber::from_bytecode(
            vec![PushConst(Val::Int(42)), Eval(false)],
            Env::standard(),
            (),
        );
        assert_eq!(f.start().unwrap(), Val::Int(42));
        assert!(f.is_done());

        let mut f = Fiber::from_bytecode(
            vec![
                PushConst(Val::List(vec![
                    Val::symbol("quote"),
                    Val::List(vec![
                        Val::keyword("a"),
                        Val::keyword("b"),
                        Val::keyword("c"),
                    ]),
                ])),
                Eval(false),
            ],
            Env::standard(),
            (),
        );

        assert_eq!(
            f.start().unwrap(),
            Val::List(vec![
                Val::keyword("a"),
                Val::keyword("b"),
                Val::keyword("c"),
            ])
        );
        assert!(f.is_done())
    }

    #[test]
    fn eval_error() {
        let mut f = Fiber::from_bytecode(
            vec![PushConst(Val::symbol("jibberish")), Eval(false)],
            Env::standard(),
            (),
        );
        assert_matches!(f.start(), Err(Error::UndefinedSymbol(_)));

        let mut f = Fiber::from_bytecode(
            vec![
                PushConst(Val::List(vec![
                    Val::symbol("nonexisting"),
                    Val::keyword("a"),
                    Val::keyword("b"),
                    Val::keyword("c"),
                ])),
                Eval(false),
            ],
            Env::standard(),
            (),
        );
        assert_matches!(f.start(), Err(Error::UndefinedSymbol(_)));
    }

    #[test]
    fn yield_loop() {
        let mut f = Fiber::from_bytecode(
            vec![PushConst(Val::string("hi")), YieldTop, PopTop, JumpBck(4)],
            Env::standard(),
            (),
        );

        assert_eq!(f.start().unwrap(), Val::string("hi"));

        assert_eq!(f.resume(Val::Nil).unwrap(), Val::string("hi"));
        assert_eq!(f.resume(Val::Nil).unwrap(), Val::string("hi"));
        assert_eq!(f.resume(Val::Nil).unwrap(), Val::string("hi"));
        assert_eq!(f.resume(Val::Nil).unwrap(), Val::string("hi"));
    }

    // // TODO: Add Test case for NativeFnOp::Call
}
