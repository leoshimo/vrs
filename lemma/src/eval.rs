//! Implements evaluation of expressions
use crate::{parse::parse, Env, Error, Form, Lambda, Result, SpecialForm, SymbolId, Value};
use tracing::debug;

/// Evaluate a given expression
pub fn eval_expr(expr: &str, env: &mut Env) -> Result<Value> {
    let form = parse(expr)?;
    eval(&form, env)
}

/// Evaluate a form within given environment
pub fn eval(form: &Form, env: &mut Env) -> Result<Value> {
    debug!("eval - {:?}", form);
    match form {
        Form::Bool(_) | Form::Int(_) | Form::String(_) | Form::Keyword(_) => {
            Ok(Value::from(form.clone()))
        }
        Form::Symbol(s) => eval_symbol(s, env),
        Form::List(l) => eval_list(l, env),
    }
}

/// Evaluate symbol forms
fn eval_symbol(symbol: &SymbolId, env: &mut Env) -> Result<Value> {
    debug!("eval_symbol - {:?}", symbol);
    match env.resolve(symbol) {
        Some(value) => Ok(value.clone()),
        None => Err(Error::UndefinedSymbol(symbol.clone())),
    }
}

/// Evaluate a list form
fn eval_list(forms: &[Form], env: &mut Env) -> Result<Value> {
    debug!("eval_list - ({:?})", forms);

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
        Value::Lambda(lambda) => eval_lambda_call(&lambda, arg_forms, env),
        Value::SpecialForm(sp_form) => eval_special_form(&sp_form, arg_forms, env),
        Value::Form(_) | Value::Vec(_) => Err(Error::InvalidOperation(op_value)),
    }
}

/// Evalute a lambda expression
pub fn eval_lambda_call(lambda: &Lambda, arg_forms: &[Form], env: &mut Env) -> Result<Value> {
    debug!("eval_lambda_call - ({:?})", lambda,);

    let arg_vals = arg_forms
        .iter()
        .map(|f| eval(f, env))
        .collect::<Result<Vec<_>>>()?;

    eval_lambda_call_vals(lambda, &arg_vals, env)
}

/// Evaluate a lambda expression, passing in values as args
pub fn eval_lambda_call_vals(lambda: &Lambda, arg_vals: &[Value], env: &mut Env) -> Result<Value> {
    if lambda.params.len() != arg_vals.len() {
        return Err(Error::UnexpectedNumberOfArguments);
    }

    // TODO: Lexical scope instead of Dynamic scope?
    let mut lambda_env = Env::extend(env);
    for (param, val) in lambda.params.iter().zip(arg_vals) {
        lambda_env.bind(param, val.clone()); // TODO: How can I clone val?
    }

    let mut res = Value::from(Form::List(vec![])); // TODO: Dedicated nil in language?
    for form in lambda.body.iter() {
        res = eval(form, &mut lambda_env)?;
    }
    Ok(res)
}

/// Evaluate a special form expression
fn eval_special_form(
    sp_form: &SpecialForm,
    arg_forms: &[Form],
    env: &mut Env<'_>,
) -> std::result::Result<Value, Error> {
    debug!("eval_special_form - {:?}", sp_form,);
    // TODO: Lexical binding?
    (sp_form.func)(arg_forms, env)
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn eval_bool() {
        let mut env = Env::new();
        assert_eq!(eval_expr("true", &mut env), Ok(Value::from(true)));
        assert_eq!(eval_expr("false", &mut env), Ok(Value::from(false)));
    }

    #[test]
    fn eval_int() {
        let mut env = Env::new();
        assert_eq!(eval_expr("5", &mut env), Ok(Value::from(5)));
    }

    #[test]
    fn eval_string() {
        let mut env = Env::new();
        assert_eq!(eval_expr("\"Hello\"", &mut env), Ok(Value::from("Hello")));
    }

    /// Eval symbols
    #[test]
    fn eval_symbols() {
        let mut env = Env::new();
        env.bind(&SymbolId::from("greeting"), Value::from("hello world"));

        assert_eq!(
            eval_expr("greeting", &mut env),
            Ok(Value::from("hello world"))
        );

        assert!(matches!(
            eval_expr("undefined", &mut env),
            Err(Error::UndefinedSymbol(_))
        ));
    }

    /// Eval list
    #[test]
    fn eval_list_empty() {
        let mut env = Env::new();
        assert_eq!(
            eval_expr("()", &mut env),
            Ok(Value::Form(Form::List(vec![])))
        );
    }

    /// Eval functions
    #[test]
    fn eval_function() {
        let mut env = Env::new();
        env.bind(
            &SymbolId::from("echo"),
            Value::Lambda(Lambda {
                params: vec![SymbolId::from("x")],
                body: vec![Form::symbol("x")],
            }),
        );

        assert!(matches!(eval_expr("echo", &mut env), Ok(Value::Lambda(_)),));

        assert_eq!(eval_expr("(echo 10)", &mut env), Ok(Value::from(10)));
    }

    /// Eval special forms
    #[test]
    fn eval_special_form() {
        let mut env = Env::new();
        env.bind_special_form(SpecialForm {
            symbol: SymbolId::from("quote"),
            func: |arg_forms, _env| Ok(Value::Form(arg_forms[0].clone())),
        });

        assert!(matches!(
            eval_expr("quote", &mut env),
            Ok(Value::SpecialForm(l)) if l.symbol == SymbolId::from("quote"),
        ));

        assert_eq!(
            eval_expr("(quote (1 2 3))", &mut env),
            Ok(Value::Form(Form::List(vec![
                Form::Int(1),
                Form::Int(2),
                Form::Int(3),
            ])))
        );
    }
}
