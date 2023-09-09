//! Implements evaluation of expressions
use crate::{Env, Error, Form, Result, SymbolId, Value};

/// Evaluate a form within given environment
pub fn eval(form: &Form, env: &Env) -> Result<Value> {
    match form {
        Form::Int(_) | Form::String(_) => Ok(Value::from(form.clone())),
        Form::Symbol(s) => eval_symbol(s, env),
        Form::List(l) => eval_list(l, env),
    }
}

/// Evaluate symbol forms
pub fn eval_symbol(symbol: &SymbolId, env: &Env) -> Result<Value> {
    match env.resolve(symbol) {
        Some(value) => Ok(value.clone()),
        None => Err(Error::UndefinedSymbol(symbol.clone())),
    }
}

/// Evaluate a list form
pub fn eval_list(forms: &[Form], env: &Env) -> Result<Value> {
    if forms.is_empty() {
        return Ok(Value::from(Form::List(vec![])));
    }
    let (op_form, _arg_forms) = forms.split_first().expect("forms is nonempty");

    let _op_value = match op_form {
        Form::Symbol(symbol) => eval_symbol(symbol, env),
        Form::List(l) => eval_list(l, env),
        _ => Err(Error::UnexpectedOperator(format!(
            "{} is not a valid operator",
            op_form
        ))),
    }?;

    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Self-evaluating forms
    #[test]
    fn eval_self_evaluating() {
        let env = Env::default();

        assert_eq!(eval(&Form::Int(5), &env), Ok(Value::Int(5)));

        assert_eq!(
            eval(&Form::String("Hello".to_string()), &env),
            Ok(Value::String("Hello".to_string()))
        );
    }

    /// Eval symbols
    #[test]
    fn eval_symbols() {
        let mut env = Env::default();
        env.bind(
            &SymbolId::from("greeting"),
            Value::String(String::from("hello world")),
        );

        assert_eq!(
            eval(&Form::symbol("greeting"), &env),
            Ok(Value::from("hello world"))
        );

        assert!(matches!(
            eval(&Form::symbol("undefined"), &env),
            Err(Error::UndefinedSymbol(_))
        ));
    }
}
