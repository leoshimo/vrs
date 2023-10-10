//! A fiber of execution that can be cooperatively scheduled via yielding.
use crate::Form;
use super::Inst;

#[derive(Debug)]
pub struct Fiber {
    cframes: Vec<CallFrame>,
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
        }


        if f.is_done() {
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
    fn is_done(&self) -> bool {
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

    #[test]
    fn fiber_load_const_return() {
        let mut f = Fiber::from_bytecode(vec![PushConst(Int(5))]);
        start(&mut f).expect("should start");
        assert_eq!(f.status, Status::Completed(Ok(Int(5))));

        let mut f = Fiber::from_bytecode(vec![PushConst(Form::string("Hi"))]);
        start(&mut f).expect("should start");
        assert_eq!(f.status, Status::Completed(Ok(Form::string("Hi"))));
    }

}
