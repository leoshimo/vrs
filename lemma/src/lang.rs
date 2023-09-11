//! Defines opt-in language constructs
//! Lemma interpreter does not have built-in procedures and special forms by default.
//! The language features are "opt in" by defining symbols within the environment

use std::rc::Rc;

use crate::{
    eval,
    value::{Lambda, LambdaFn, SpecialForm},
    Env, Error, Form, Result, SymbolId, Value,
};

/// Returns the 'standard' environment of the langugae
pub fn std_env<'a>() -> Env<'a> {
    let mut env = Env::new();
    add_lambda(&mut env);
    add_quote(&mut env);
    add_eval(&mut env);
    env
}

/// Adds `lambda` symbol for creating functions
/// `(lambda (PARAMS*) FORM)`
pub fn add_lambda(env: &mut Env<'_>) {
    let lambda_sym = SymbolId::from("lambda");
    env.bind(
        &lambda_sym,
        Value::SpecialForm(SpecialForm {
            name: lambda_sym.to_string(),
            func: lambda,
        }),
    );
}

/// Adds `quote` symbol for quoting forms
pub fn add_quote(env: &mut Env) {
    let quote_sym = SymbolId::from("quote");
    env.bind(
        &quote_sym,
        Value::SpecialForm(SpecialForm {
            name: quote_sym.to_string(),
            func: quote,
        }),
    );
}

/// Adds the `eval` symbol for evaluating forms
pub fn add_eval(env: &mut Env) {
    let eval_sym = SymbolId::from("eval");
    env.bind(
        &eval_sym,
        Value::SpecialForm(SpecialForm {
            name: eval_sym.to_string(),
            func: eval,
        }),
    );
}

/// Implements `lambda` special form
fn lambda(arg_forms: &[Form], _env: &mut Env) -> Result<Value> {
    let (params, body) = arg_forms
        .split_first()
        .ok_or(Error::MissingLambdaParameterList)?;

    let params = match params {
        Form::List(l) => Ok(l),
        _ => Err(Error::MissingLambdaParameterList),
    }?;

    let params = params
        .iter()
        .map(|p| match p {
            Form::Symbol(s) => Ok(s.clone()),
            _ => Err(Error::ParameterListContainsNonSymbol),
        })
        .collect::<Result<Vec<_>>>()?;

    let body = body.to_owned();
    let func: LambdaFn = Rc::new(move |env| {
        let mut res = Value::from(Form::List(vec![])); // TODO: Dedicated nil in language?
        for form in body.iter() {
            res = eval::eval(form, env)?;
        }
        Ok(res)
    });

    Ok(Value::Func(Lambda { params, func }))
}

/// Implements the `quote` special form
fn quote(arg_forms: &[Form], _env: &mut Env) -> Result<Value> {
    if arg_forms.len() == 1 {
        Ok(Value::Form(arg_forms[0].clone()))
    } else {
        Err(Error::QuoteExpectsSingleArgument)
    }
}

/// Implements the `eval` special form
fn eval(arg_forms: &[Form], env: &mut Env) -> Result<Value> {
    let arg_form = match arg_forms {
        [form] => Ok(form),
        _ => Err(Error::EvalExpectsSingleFormArgument),
    }?;

    match eval::eval(arg_form, env)? {
        Value::Form(f) => eval::eval(&f, env),
        _ => Err(Error::EvalExpectsSingleFormArgument),
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::eval_expr;
    use tracing_test::traced_test;

    #[test]
    #[traced_test]
    fn eval_lambda() {
        let mut env = std_env();

        assert!(
            matches!(eval_expr("lambda", &mut env), Ok(Value::SpecialForm(_))),
            "lambda symbol should be defined"
        );

        assert!(
            matches!(
                eval_expr(
                    "(lambda (x y) 10)",
                    &mut env
                ),
                Ok(Value::Func(Lambda { params, .. })) if params == vec![SymbolId::from("x"), SymbolId::from("y")]
            ),
            "lambda special form returns a lambda value"
        );

        // ((lambda (x) x) 5) => 5
        assert_eq!(eval_expr("((lambda (x) x) 5)", &mut env), Ok(Value::Int(5)));

        // ((lambda () (lambda (x) x))) => Value::Func
        assert!(matches!(
            eval_expr("((lambda () (lambda (x) x)))", &mut env),
            Ok(Value::Func(_))
        ));

        // (((lambda () (lambda (x) x))) 10) => 10
        assert_eq!(
            eval_expr("(((lambda () (lambda (x) x))) 10)", &mut env),
            Ok(Value::Int(10))
        );
    }

    #[test]
    #[traced_test]
    fn eval_quote() {
        let mut env = std_env();

        assert_eq!(
            eval_expr("(quote (one :two three))", &mut env),
            Ok(Value::Form(Form::List(vec![
                Form::symbol("one"),
                Form::keyword("two"),
                Form::symbol("three"),
            ])))
        );

        assert_eq!(
            eval_expr("(quote (lambda (x) x))", &mut env),
            Ok(Value::Form(Form::List(vec![
                Form::symbol("lambda"),
                Form::List(vec![Form::symbol("x")]),
                Form::symbol("x"),
            ])))
        );

        assert!(
            matches!(
                eval_expr("((quote (lambda (x) x)) 5)", &mut env),
                Err(Error::InvalidOperation(Value::Form(_)))
            ),
            "A quoted operation does not recursively evaluate without explicit call to eval"
        );
    }

    #[test]
    #[traced_test]
    fn eval_eval() {
        let mut env = std_env();

        assert_eq!(eval_expr("(eval (quote 5))", &mut env), Ok(Value::Int(5)));

        assert_eq!(
            eval_expr("(eval (quote ((lambda (x) x) 5)))", &mut env),
            Ok(Value::Int(5))
        );
    }
}
