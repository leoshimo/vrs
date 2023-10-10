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
pub enum CompileError {
    #[error("Invalid expression - {0}")]
    InvalidExpression(String),
}

pub type Result<T> = std::result::Result<T, CompileError>;

/// Compile expression
pub fn compile(f: &Form) -> Result<Vec<Inst>> {
    match f {
        Form::List(l) => {
            let (first, args) = l.split_first().ok_or(CompileError::InvalidExpression(
                "Empty list expression".to_string(),
            ))?;

            match first {
                Form::Symbol(s) => match s.as_str() {
                    "def" => compile_def(args),
                    _ => todo!(),
                },
                _ => todo!(),
            }
        }
        Form::Symbol(s) => Ok(vec![Inst::LoadSym(s.clone())]),
        _ => Ok(vec![Inst::PushConst(f.clone())]),
    }
}

/// Compile special form builtin def
fn compile_def(args: &[Form]) -> Result<Vec<Inst>> {
    let (symbol, value) = match args {
        [Form::Symbol(symbol), value] => (symbol, value),
        _ => {
            return Err(CompileError::InvalidExpression(
                "def accepts one symbol and one form as arguments".to_string(),
            ))
        }
    };

    let mut inst = compile(value)?;
    inst.push(Inst::StoreSym(symbol.clone()));
    Ok(inst)
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

    #[test]
    fn compile_empty_list() {
        assert!(matches!(
            compile(&f("()")),
            Err(CompileError::InvalidExpression(_))
        ))
    }

    #[test]
    fn compile_def() {
        assert_eq!(
            compile(&f("(def x 5)")),
            Ok(vec![PushConst(Form::Int(5)), StoreSym(SymbolId::from("x")),])
        );
    }

    /// Convenience for creating Forms from strs
    fn f(expr: &str) -> Form {
        parse(expr).expect("expr should be valid form")
    }
}
