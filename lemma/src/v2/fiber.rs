//! A fiber of execution that can be cooperatively scheduled via yielding.
use super::Opcode;
use super::Form;

#[derive(Debug)]
pub struct Fiber {
    /// Call frames for Fiber. Last is top of callframe
    cframes: Vec<CallFrame>,
    /// Data stack
    stack: Vec<Form>,
}

/// Status for a fiber
#[derive(Debug, Clone)]
pub enum Status {
    New,
    Running,
    Completed(Form),
}

#[derive(Debug)]
struct CallFrame {
    ip: usize,
    code: Vec<Opcode>,
}

/// Start or Resume execution of a fiber
pub fn resume(_f: &mut Fiber, _form: Form) -> Status {
    todo!()
}
