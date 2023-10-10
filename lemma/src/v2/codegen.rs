//! Compiler for Lemma Form AST
use super::Form;
use serde::{Deserialize, Serialize};

// TODO: Compact bytecode repr
/// Bytecode instructions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Inst {
    /// Push constant form onto stack
    PushConst(Form),
}

/// Errors during compilation
#[derive(thiserror::Error, Debug, PartialEq)]
pub enum CompileError {}

pub type Result<T> = std::result::Result<T, CompileError>;

/// Compile expression
pub fn compile(f: &Form) -> Result<Vec<Inst>> {
    match f {
        Form::Int(_) | Form::String(_) => {
            Ok(vec![
                Inst::PushConst(f.clone())
            ])
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use super::Inst::*;

    #[test]
    fn compile_self_evaluating() {
        assert_eq!(compile(&Form::string("Hello")), Ok(vec![
            PushConst(Form::string("Hello")),
        ]));
    }
}
