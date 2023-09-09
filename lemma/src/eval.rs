//! Implements evaluation of expressions
use crate::env::Binding;
use crate::{Env, Error, Expr, Result};

/// Evaluate a form within given environment
pub fn eval(form: &Expr, env: &Env) -> Result<Expr> {
    match form {
        Expr::Int(_) | Expr::String(_) => Ok(form.clone()),
        Expr::Symbol(s) => eval_symbol(s, env),
        Expr::List(_l) => todo!(),
    }
}

/// Evaluate symbol forms
pub fn eval_symbol(symbol: &String, env: &Env) -> Result<Expr> {
    match env.resolve(symbol) {
        Some(Binding::Normal(expr)) => Ok(expr.clone()),
        Some(Binding::Special(_special_fn)) => {
            todo!()
        }
        None => Err(Error::UndefinedSymbol(symbol.to_string())),
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

    /// Eval symbols
    #[test]
    fn eval_symbols() {
        let mut env = Env::default();
        env.bind("greeting", Expr::String(String::from("hello world")));

        assert_eq!(
            eval(&Expr::Symbol(String::from("greeting")), &env),
            Ok(Expr::String(String::from("hello world")))
        );

        assert!(matches!(
            eval(&Expr::Symbol(String::from("undefined")), &env),
            Err(Error::UndefinedSymbol(_))
        ));
    }
}
