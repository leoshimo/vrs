//! Implements evaluation of expressions
use crate::env::Binding;
use crate::{Env, Error, Form, Result};

/// Evaluate a form within given environment
pub fn eval(form: &Form, env: &Env) -> Result<Form> {
    match form {
        Form::Int(_) | Form::String(_) => Ok(form.clone()),
        Form::Symbol(s) => eval_symbol(s, env),
        Form::List(l) => eval_list(l, env),
    }
}

/// Evaluate symbol forms
pub fn eval_symbol(symbol: &String, env: &Env) -> Result<Form> {
    match env.resolve(symbol) {
        Some(Binding::Normal(form)) => Ok(form.clone()),
        None => Err(Error::UndefinedSymbol(symbol.to_string())),
        _ => todo!(),
    }
}

/// Evaluate a list form
pub fn eval_list(forms: &[Form], env: &Env) -> Result<Form> {
    if forms.is_empty() {
        return Ok(Form::List(vec![]));
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

        assert_eq!(eval(&Form::Int(5), &env), Ok(Form::Int(5)));

        assert_eq!(
            eval(&Form::String("Hello".to_string()), &env),
            Ok(Form::String("Hello".to_string()))
        );
    }

    /// Eval symbols
    #[test]
    fn eval_symbols() {
        let mut env = Env::default();
        env.bind("greeting", Form::String(String::from("hello world")));

        assert_eq!(
            eval(&Form::Symbol(String::from("greeting")), &env),
            Ok(Form::String(String::from("hello world")))
        );

        assert!(matches!(
            eval(&Form::Symbol(String::from("undefined")), &env),
            Err(Error::UndefinedSymbol(_))
        ));
    }
}
