//! Compiler for Lemma Form AST
use super::Form;
use serde::{Deserialize, Serialize};

// TODO: Smaller size
/// Bytecode instructions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Inst {
    /// Push constant form onto stack
    PushConst(Form),
    /// Unwinds call frame, pushing current TOS to stack as result
    Ret,
}
