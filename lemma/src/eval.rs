//! Implements evaluation of expressions
use crate::{Expr, Result};

/// The Lemma environment that has all bindings
#[derive(Debug, Default)]
pub struct Env {}

/// Evaluate a form within given environment
pub fn eval(form: &Expr, env: &Env) -> Result<Expr> {
    match form {
        Expr::Int(_) | Expr::String(_) => Ok(form.clone()),
        Expr::Symbol(_) => todo!(),
        Expr::List(_) => todo!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Self-evaluating forms
    #[test]
    fn eval_self_evaluating() {
        let env = Env::default();
        assert_eq!(eval(&Expr::Int(5), &env), Ok(Expr::Int(5)));
        assert_eq!(
            eval(&Expr::String("Hello".to_string()), &env),
            Ok(Expr::String("Hello".to_string()))
        );
    }
}
