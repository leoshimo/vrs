//! Implements evaluation of expressions
use crate::{parse::parse, value::Lambda, Env, Error, Form, Result, SymbolId, Value};

/// Evaluate a given expression
pub fn eval_expr(expr: &str) -> Result<Value> {
    let form = parse(expr)?;
    let env = Env::default();
    eval(&form, &env)
}

/// Evaluate a form within given environment
pub(crate) fn eval(form: &Form, env: &Env) -> Result<Value> {
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

    let (op_form, arg_forms) = forms.split_first().expect("forms is nonempty");

    let op_value = match op_form {
        Form::Symbol(symbol) => eval_symbol(symbol, env),
        Form::List(l) => eval_list(l, env),
        _ => Err(Error::UnexpectedOperator(format!(
            "{} is not a valid operator",
            op_form
        ))),
    }?;

    match op_value {
        Value::Function { name, lambda } => eval_func_call(&name, lambda, arg_forms, env),
        Value::Int(_) | Value::String(_) | Value::Form(_) => Err(Error::InvalidOperation(op_value)),
    }
}

/// Evalute a function call
pub fn eval_func_call(_name: &str, lambda: Lambda, arg_forms: &[Form], env: &Env) -> Result<Value> {
    let arg_vals = arg_forms
        .iter()
        .map(|f| eval(f, env))
        .collect::<Result<Vec<_>>>()?;

    // TODO: Function local scope
    // TODO: Parameter bindings

    (lambda)(&arg_vals)
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

    /// Eval list
    #[test]
    fn eval_list_empty() {
        let env = Env::default();
        assert_eq!(
            eval(&Form::List(vec![]), &env),
            Ok(Value::Form(Form::List(vec![])))
        );
    }

    /// Eval functions
    #[test]
    fn eval_function() {
        let mut env = Env::default();
        env.bind(
            &SymbolId::from("add"),
            Value::Function {
                name: String::from("add"),
                lambda: |args| match (&args[0], &args[1]) {
                    (Value::Int(x), Value::Int(y)) => Ok(Value::Int(x + y)),
                    _ => Err(Error::InvalidArgumentsToFunctionCall),
                },
            },
        );

        assert!(matches!(
            eval(&Form::symbol("add"), &env),
            Ok(Value::Function { name, ..}) if name == "add",
        ));

        assert_eq!(
            eval(
                &Form::List(vec![Form::symbol("add"), Form::Int(10), Form::Int(2)]),
                &env
            ),
            Ok(Value::Int(12))
        );
    }
}
