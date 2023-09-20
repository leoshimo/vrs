//! Implements evaluation of expressions
use crate::{parse::parse, Env, Error, Form, Lambda, NativeFunc, Result, SymbolId};
use tracing::debug;

/// Evaluate a given expression
pub fn eval_expr(expr: &str, env: &mut Env) -> Result<Form> {
    let form = parse(expr)?;
    eval(&form, env)
}

/// Evaluate a form within given environment
pub fn eval(form: &Form, env: &mut Env) -> Result<Form> {
    debug!("eval - {:?}", form);
    match form {
        Form::Symbol(s) => eval_symbol(s, env),
        Form::List(l) => eval_list(l, env),
        _ => Ok(form.clone()),
    }
}

/// Evaluate symbol forms
fn eval_symbol(symbol: &SymbolId, env: &mut Env) -> Result<Form> {
    debug!("eval_symbol - {:?}", symbol);
    match env.resolve(symbol) {
        Some(value) => Ok(value.clone()),
        None => Err(Error::UndefinedSymbol(symbol.clone())),
    }
}

/// Evaluate a list form
fn eval_list(forms: &[Form], env: &mut Env) -> Result<Form> {
    debug!("eval_list - ({:?})", forms);

    if forms.is_empty() {
        return Err(Error::MissingProcedure);
    }

    let (op_form, arg_forms) = forms.split_first().expect("forms is nonempty");

    match eval(op_form, env)? {
        Form::Lambda(lambda) => eval_lambda_call(&lambda, arg_forms, env),
        Form::NativeFunc(sp_form) => eval_special_form(&sp_form, arg_forms, env),
        _ => Err(Error::NotAProcedure(op_form.clone())),
    }
}

/// Evalute a lambda expression
pub fn eval_lambda_call(lambda: &Lambda, arg_forms: &[Form], env: &mut Env) -> Result<Form> {
    debug!("eval_lambda_call - ({:?})", lambda,);

    let arg_vals = arg_forms
        .iter()
        .map(|f| eval(f, env))
        .collect::<Result<Vec<_>>>()?;

    eval_lambda_call_vals(lambda, &arg_vals, env)
}

/// Evaluate a lambda expression, passing in values as args
pub fn eval_lambda_call_vals(lambda: &Lambda, arg_vals: &[Form], env: &mut Env) -> Result<Form> {
    if lambda.params.len() != arg_vals.len() {
        return Err(Error::UnexpectedArguments(format!(
            "expected {} arguments - got {}",
            lambda.params.len(),
            arg_vals.len()
        )));
    }

    // TODO: Lexical scope instead of Dynamic scope?
    let mut lambda_env = Env::extend(env);
    for (param, val) in lambda.params.iter().zip(arg_vals) {
        lambda_env.bind(param, val.clone());
    }

    let mut res = Form::Nil;
    for form in lambda.body.iter() {
        res = eval(form, &mut lambda_env)?;
    }
    Ok(res)
}

/// Evaluate a special form expression
fn eval_special_form(
    sp_form: &NativeFunc,
    arg_forms: &[Form],
    env: &mut Env<'_>,
) -> std::result::Result<Form, Error> {
    debug!("eval_special_form - {:?}", sp_form,);
    // TODO: Lexical binding?
    (sp_form.func)(arg_forms, env)
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::Form as F;

    #[test]
    fn eval_bool() {
        let mut env = Env::new();
        assert_eq!(eval_expr("true", &mut env), Ok(F::Bool(true)));
        assert_eq!(eval_expr("false", &mut env), Ok(F::Bool(false)));
    }

    #[test]
    fn eval_int() {
        let mut env = Env::new();
        assert_eq!(eval_expr("5", &mut env), Ok(F::Int(5)));
    }

    #[test]
    fn eval_string() {
        let mut env = Env::new();
        assert_eq!(eval_expr("\"Hello\"", &mut env), Ok(F::string("Hello")));
    }

    /// Eval symbols
    #[test]
    fn eval_symbols() {
        let mut env = Env::new();
        env.bind(&SymbolId::from("greeting"), F::string("hello world"));

        assert_eq!(
            eval_expr("greeting", &mut env),
            Ok(F::string("hello world"))
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
        assert_eq!(eval_expr("()", &mut env), Err(Error::MissingProcedure),);
    }

    /// Eval functions
    #[test]
    fn eval_function() {
        let mut env = Env::new();
        env.bind(
            &SymbolId::from("echo"),
            F::Lambda(Lambda {
                params: vec![SymbolId::from("x")],
                body: vec![Form::symbol("x")],
            }),
        );

        assert!(matches!(eval_expr("echo", &mut env), Ok(F::Lambda(_)),));

        assert_eq!(eval_expr("(echo 10)", &mut env), Ok(F::Int(10)));
    }

    /// Eval special forms
    #[test]
    fn eval_special_form() {
        let mut env = Env::new();
        env.bind_native(NativeFunc {
            symbol: SymbolId::from("quote"),
            func: |arg_forms, _env| Ok(arg_forms[0].clone()),
        });

        assert!(matches!(
            eval_expr("quote", &mut env),
            Ok(F::NativeFunc(l)) if l.symbol == SymbolId::from("quote"),
        ));

        assert_eq!(
            eval_expr("(quote (1 2 3))", &mut env),
            Ok(F::List(vec![F::Int(1), F::Int(2), F::Int(3),]))
        );
    }
}
