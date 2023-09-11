//! Implements evaluation of expressions
use crate::{
    parse::parse,
    value::{Lambda, SpecialForm},
    Env, Error, Form, Result, SymbolId, Value,
};
use tracing::debug;

/// Evaluate a given expression
pub fn eval_expr(expr: &str, env: &mut Env) -> Result<Value> {
    let form = parse(expr)?;
    eval(&form, env)
}

/// Evaluate a form within given environment
pub(crate) fn eval(form: &Form, env: &mut Env) -> Result<Value> {
    debug!("eval - {:?}", form);
    match form {
        Form::Int(_) | Form::String(_) | Form::Keyword(_) => Ok(Value::from(form.clone())),
        Form::Symbol(s) => eval_symbol(s, env),
        Form::List(l) => eval_list(l, env),
    }
}

/// Evaluate symbol forms
pub fn eval_symbol(symbol: &SymbolId, env: &mut Env) -> Result<Value> {
    debug!("eval_symbol - {:?}", symbol);
    match env.resolve(symbol) {
        Some(value) => Ok(value.clone()),
        None => Err(Error::UndefinedSymbol(symbol.clone())),
    }
}

/// Evaluate a list form
pub fn eval_list(forms: &[Form], env: &mut Env) -> Result<Value> {
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
        Value::Func(lambda) => eval_func_call(&lambda, arg_forms, env),
        Value::SpecialForm(sp_form) => eval_special_form(&sp_form, arg_forms, env),
        Value::Int(_) | Value::String(_) | Value::Keyword(_) | Value::Form(_) => {
            Err(Error::InvalidOperation(op_value))
        }
    }
}

/// Evalute a function call
pub fn eval_func_call(lambda: &Lambda, arg_forms: &[Form], env: &mut Env) -> Result<Value> {
    debug!("eval_func_call - ({:?})", lambda,);

    let arg_vals = arg_forms
        .iter()
        .map(|f| eval(f, env))
        .collect::<Result<Vec<_>>>()?;

    if lambda.params.len() != arg_forms.len() {
        return Err(Error::UnexpectedNumberOfArguments);
    }

    // TODO: Lexical scope instead of Dynamic scope?
    let mut func_env = Env::extend(env);
    for (param, val) in lambda.params.iter().zip(arg_vals) {
        func_env.bind(param, val);
    }

    (lambda.func)(&mut func_env)
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
    use std::rc::Rc;

    use super::*;

    /// Self-evaluating forms
    #[test]
    fn eval_self_evaluating() {
        let mut env = Env::new();
        assert_eq!(eval_expr("5", &mut env), Ok(Value::Int(5)));
        assert_eq!(
            eval_expr("\"Hello\"", &mut env),
            Ok(Value::String("Hello".to_string()))
        );
    }

    /// Eval symbols
    #[test]
    fn eval_symbols() {
        let mut env = Env::new();
        env.bind(
            &SymbolId::from("greeting"),
            Value::String(String::from("hello world")),
        );

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
            &SymbolId::from("add"),
            Value::Func(Lambda {
                params: vec![SymbolId::from("x"), SymbolId::from("y")],
                func: Rc::new(|env| {
                    match (
                        env.resolve(&SymbolId::from("x")),
                        env.resolve(&SymbolId::from("y")),
                    ) {
                        (Some(Value::Int(x)), Some(Value::Int(y))) => Ok(Value::Int(x + y)),
                        _ => Err(Error::InvalidArgumentsToFunctionCall),
                    }
                }),
            }),
        );

        assert!(matches!(eval_expr("add", &mut env), Ok(Value::Func(_)),));

        assert_eq!(eval_expr("(add 10 2)", &mut env), Ok(Value::Int(12)));
    }

    /// Eval special forms
    #[test]
    fn eval_special_form() {
        let mut env = Env::new();
        env.bind(
            &SymbolId::from("quote"),
            Value::SpecialForm(SpecialForm {
                name: String::from("quote"),
                func: |arg_forms, _env| Ok(Value::Form(arg_forms[0].clone())),
            }),
        );

        assert!(matches!(
            eval_expr("quote", &mut env),
            Ok(Value::SpecialForm(l)) if l.name == "quote",
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
