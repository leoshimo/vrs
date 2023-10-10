//! A fiber of execution that can be cooperatively scheduled via yielding.
use super::Form;
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
        let inst = f.next_inst().clone();
        match inst {
            Inst::PushConst(form) => {
                f.stack.push(form);
            }
            Inst::Ret => {
                let res = f.stack.pop().expect("Stack should contain result");
                f.status = Status::Completed(Ok(res));
                break;
            }
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

    /// Get the next instruction, incrementing the instruction pointer
    pub(crate) fn next_inst(&mut self) -> &Inst {
        let top = self.cframes.last_mut().expect("Fiber has no callframes!");
        let inst = &top
            .code
            .get(top.ip)
            .expect("Callstack executing outside bytecode length");
        top.ip += 1;
        inst
    }
}

impl CallFrame {
    /// Create a new callframe for executing given bytecode from start
    pub fn from_bytecode(code: Vec<Inst>) -> Self {
        Self { ip: 0, code }
    }
}

#[cfg(test)]
mod tests {
    use super::Form::*;
    use super::Inst::*;
    use super::*;

    #[test]
    fn fiber_load_const_return() {
        let mut f = Fiber::from_bytecode(vec![PushConst(Int(5)), Ret]);
        start(&mut f).expect("should start");
        assert_eq!(f.status, Status::Completed(Ok(Int(5))));

        let mut f = Fiber::from_bytecode(vec![PushConst(Form::string("Hi")), Ret]);
        start(&mut f).expect("should start");
        assert_eq!(f.status, Status::Completed(Ok(Form::string("Hi"))));
    }

    // TODO: Test: Report exceeding callstack bytecode OR popping off callstack is... propagated maybe?
}
