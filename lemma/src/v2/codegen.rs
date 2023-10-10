//! Compiler for Lemma Form AST
use crate::{Form, SymbolId};
use serde::{Deserialize, Serialize};

// TODO: Compact bytecode repr
/// Bytecode instructions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Inst {
    /// Push constant form onto stack
    PushConst(Form),
    /// Push value bound to given symbol onto stack
    LoadSym(SymbolId),
    /// Pop TOS and store value as given symbol
    StoreSym(SymbolId),
}

/// Errors during compilation
#[derive(thiserror::Error, Debug, PartialEq)]
pub enum CompileError {}

pub type Result<T> = std::result::Result<T, CompileError>;

/// Compile expression
pub fn compile(f: &Form) -> Result<Vec<Inst>> {
    match f {
        Form::Symbol(s) => Ok(vec![Inst::LoadSym(s.clone())]),
        Form::List(_l) => todo!(),
        _ => Ok(vec![Inst::PushConst(f.clone())]),
    }
}

#[cfg(test)]
mod tests {
    use super::Inst::*;
    use super::*;
    use crate::parse;

    #[test]
    fn compile_self_evaluating() {
        assert_eq!(compile(&Form::Int(10)), Ok(vec![PushConst(Form::Int(10)),]));
        assert_eq!(
            compile(&Form::string("Hello")),
            Ok(vec![PushConst(Form::string("Hello")),])
        );
    }

    #[test]
    fn compile_symbol() {
        assert_eq!(
            compile(&Form::symbol("x")),
            Ok(vec![LoadSym(SymbolId::from("x"))])
        );
    }

    /// Convenience for creating Forms from strs
    fn f(expr: &str) -> Form {
        parse(expr).expect("expr should be valid form")
    }
}
