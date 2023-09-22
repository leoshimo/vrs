//! Math operations for lemma
use crate::eval::eval;
use crate::{Env, Error, Form, Result};

// TODO: Compress implementations

/// Implements the + operator
pub fn lang_add(forms: &[Form], env: &mut Env) -> Result<Form> {
    // TODO: Support N operands?
    let (lhs, rhs) = match forms {
        [lhs, rhs] => Ok((lhs, rhs)),
        _ => Err(Error::UnexpectedArguments(
            "add expects two arguments".to_string(),
        )),
    }?;

    match (eval(lhs, env)?, eval(rhs, env)?) {
        (Form::Int(lhs), Form::Int(rhs)) => Ok(Form::Int(lhs + rhs)),
        _ => Err(Error::UnexpectedArguments(
            "add expects two integers".to_string(),
        )),
    }
}

pub fn lang_less(forms: &[Form], env: &mut Env) -> Result<Form> {
    let (lhs, rhs) = match forms {
        [lhs, rhs] => Ok((lhs, rhs)),
        _ => Err(Error::UnexpectedArguments(
            "add expects two arguments".to_string(),
        )),
    }?;

    match (eval(lhs, env)?, eval(rhs, env)?) {
        (Form::Int(lhs), Form::Int(rhs)) => Ok(Form::Bool(lhs < rhs)),
        _ => Err(Error::UnexpectedArguments(
            "add expects two integers".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::eval_expr;
    use crate::lang::std_env;

    #[test]
    fn eval_add() {
        let mut env = std_env();

        assert_eq!(eval_expr("(+ 3 4)", &mut env), Ok(Form::Int(7)));

        assert_eq!(
            eval_expr("(+ (+ 1 2) (+ 3 (+ 4 5)))", &mut env),
            Ok(Form::Int(15))
        );
    }

    #[test]
    fn eval_less() {
        let mut env = std_env();

        assert_eq!(eval_expr("(< 3 4)", &mut env), Ok(Form::Bool(true)));
        assert_eq!(eval_expr("(< 500 4)", &mut env), Ok(Form::Bool(false)));
    }
}
