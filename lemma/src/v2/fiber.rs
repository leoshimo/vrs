//! A fiber of execution that can be cooperatively scheduled via yielding.
use super::{Env, Inst};
use crate::{Form, SymbolId};

#[derive(Debug)]
pub struct Fiber {
    cframes: Vec<CallFrame>,
    env: Env,
    stack: Vec<Form>,
    status: Status,
}

/// Status for a fiber
#[derive(Debug, PartialEq)]
pub enum Status {
    New,
    Running,
    Yielded,
    Completed(Result<Form>),
}

/// Single call frame
#[derive(Debug)]
struct CallFrame {
    ip: usize,
    code: Vec<Inst>,
}

/// Errors encountered during execution of expression
#[derive(thiserror::Error, Debug, PartialEq)]
pub enum FiberError {
    /// A fiber that is already running was asked to start
    #[error("Fiber is already running")]
    AlreadyRunning,

    /// Setting undefined symbol
    #[error("Undefined symbol - {0}")]
    UndefinedSymbol(SymbolId),

    /// Unexpected state on stack
    #[error("Unexpected stack state - {0}")]
    UnexpectedStack(String),
}

/// Result type for fiber ops
pub type Result<T> = std::result::Result<T, FiberError>;

/// Start execution of a fiber
pub fn start(f: &mut Fiber) -> Result<()> {
    if f.status != Status::New {
        return Err(FiberError::AlreadyRunning);
    }
    run(f);
    Ok(())
}

/// Run the fiber until it completes or yields
fn run(f: &mut Fiber) {
    f.status = Status::Running;

    while f.status == Status::Running {
        let inst = f.inst().clone(); // TODO: Use by ref until values need cloning
        f.top_mut().ip += 1;

        match inst {
            Inst::PushConst(form) => {
                f.stack.push(form);
            }
            Inst::StoreSym(s) => match f.stack.last() {
                Some(value) => f.env.define(&s, value.clone()),
                None => {
                    f.status = Status::Completed(Err(FiberError::UnexpectedStack(
                        "Expected stack to be nonempty".to_string(),
                    )))
                }
            },
            Inst::LoadSym(s) => match f.env.get(&s) {
                Some(value) => f.stack.push(value.clone()),
                None => {
                    f.status = Status::Completed(Err(FiberError::UndefinedSymbol(s)));
                }
            },
        }

        if f.status == Status::Running && f.no_more_inst() {
            let res = f.stack.pop().expect("Stack should contain result");
            f.status = Status::Completed(Ok(res));
        }
    }
}

impl Fiber {
    /// Create a new fiber from given bytecode
    pub fn from_bytecode(bytecode: Vec<Inst>) -> Self {
        Fiber {
            stack: vec![],
            cframes: vec![CallFrame::from_bytecode(bytecode)],
            env: Env::new(),
            status: Status::New,
        }
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

    /// Whether or not fiber has executed last instruction
    fn no_more_inst(&self) -> bool {
        self.cframes.len() == 1 && self.cframes.last().unwrap().is_done()
    }
}

impl CallFrame {
    /// Create a new callframe for executing given bytecode from start
    pub fn from_bytecode(code: Vec<Inst>) -> Self {
        Self { ip: 0, code }
    }

    /// Whether or not call frame should return (i.e. on last call frame execution)
    pub fn is_done(&self) -> bool {
        self.ip == self.code.len()
    }
}

#[cfg(test)]
mod tests {
    use super::Form::*;
    use super::Inst::*;
    use super::*;
    use assert_matches::assert_matches;

    #[test]
    fn fiber_load_const_return() {
        let mut f = Fiber::from_bytecode(vec![PushConst(Int(5))]);
        start(&mut f).expect("should start");
        assert_eq!(f.status, Status::Completed(Ok(Int(5))));

        let mut f = Fiber::from_bytecode(vec![PushConst(Form::string("Hi"))]);
        start(&mut f).expect("should start");
        assert_eq!(f.status, Status::Completed(Ok(Form::string("Hi"))));
    }

    #[test]
    fn store_symbol() {
        // fiber for (def x 5)
        let mut f =
            Fiber::from_bytecode(vec![PushConst(Form::Int(5)), StoreSym(SymbolId::from("x"))]);

        start(&mut f).expect("should start");

        assert_eq!(
            f.env.get(&SymbolId::from("x")),
            Some(Form::Int(5)),
            "symbol should be defined in env"
        );
        assert_eq!(
            f.status,
            Status::Completed(Ok(Form::Int(5))),
            "should complete with value of stored symbol"
        );
    }

    #[test]
    fn load_symbol() {
        let mut f = Fiber::from_bytecode(vec![LoadSym(SymbolId::from("x"))]);
        f.env.define(&SymbolId::from("x"), Form::string("hi"));

        start(&mut f).unwrap();
        assert_eq!(f.status, Status::Completed(Ok(Form::string("hi"))));
    }

    #[test]
    fn load_symbol_undefined() {
        let mut f = Fiber::from_bytecode(vec![LoadSym(SymbolId::from("x"))]);

        start(&mut f).unwrap();
        assert_matches!(
            f.status,
            Status::Completed(Err(FiberError::UndefinedSymbol(_)))
        );
    }

    // TODO: Store / Load symbol w/ lexical scopes
}
