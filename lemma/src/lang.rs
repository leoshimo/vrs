//! Defines opt-in language constructs
//! Lemma interpreter does not have built-in procedures and special forms by default.
//! The language features are "opt in" by defining symbols within the environment

use std::rc::Rc;

use crate::{
    eval::eval,
    value::{Lambda, LambdaFn, SpecialForm},
    Env, Error, Form, Result, SymbolId, Value,
};

/// Returns the 'standard' environment of the langugae
pub fn std_env<'a>() -> Env<'a> {
    let mut env = Env::new();
    add_lambda(&mut env);
    add_quote(&mut env);
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

/// Implements `lambda` special form
fn lambda(arg_forms: &[Form], _env: &Env) -> Result<Value> {
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
            res = eval(form, env)?;
        }
        Ok(res)
    });

    Ok(Value::Func(Lambda { params, func }))
}

/// Implements the `quote` special form
fn quote(arg_forms: &[Form], _env: &Env) -> Result<Value> {
    if arg_forms.len() == 1 {
        Ok(Value::Form(arg_forms[0].clone()))
    } else {
        Err(Error::QuoteExpectsSingleArgument)
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::eval_expr;

    #[test]
    fn lambda() {
        let env = std_env();

        assert!(
            matches!(eval_expr("lambda", &env), Ok(Value::SpecialForm(_))),
            "lambda symbol should be defined"
        );

        assert!(
            matches!(
                eval_expr(
                    "(lambda (x y) 10)",
                    &env
                ),
                Ok(Value::Func(Lambda { params, .. })) if params == vec![SymbolId::from("x"), SymbolId::from("y")]
            ),
            "lambda special form returns a lambda value"
        );

        // ((lambda (x) x) 5) => 5
        assert_eq!(eval_expr("((lambda (x) x) 5)", &env), Ok(Value::Int(5)));

        // ((lambda () (lambda (x) x))) => Value::Func
        assert!(matches!(
            eval_expr("((lambda () (lambda (x) x)))", &env),
            Ok(Value::Func(_))
        ));

        // (((lambda () (lambda (x) x))) 10) => 10
        assert_eq!(
            eval_expr("(((lambda () (lambda (x) x))) 10)", &env),
            Ok(Value::Int(10))
        );
    }

    #[test]
    fn quote() {
        let env = std_env();

        assert_eq!(
            eval_expr("(quote (one :two three))", &env),
            Ok(Value::Form(Form::List(vec![
                Form::symbol("one"),
                Form::keyword("two"),
                Form::symbol("three"),
            ])))
        );

        assert_eq!(
            eval_expr("(quote (lambda (x) x))", &env),
            Ok(Value::Form(Form::List(vec![
                Form::symbol("lambda"),
                Form::List(vec![Form::symbol("x")]),
                Form::symbol("x"),
            ])))
        );

        assert!(
            matches!(
                dbg!(eval_expr("((quote (lambda (x) x)) 5)", &env)),
                Err(Error::InvalidOperation(Value::Form(_)))
            ),
            "A quoted operation does not recursively evaluate without explicit call to eval"
        );
    }
}
