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
    env
}

/// Adds `lambda` keyword for defining functions:
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

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn lambda() {
        let env = std_env();

        assert!(
            matches!(
                eval(&Form::symbol("lambda"), &env),
                Ok(Value::SpecialForm(_))
            ),
            "lambda should be defined"
        );

        assert!(
            matches!(
                eval(
                    &Form::List(vec![
                        Form::symbol("lambda"),
                        Form::List(vec![
                            Form::symbol("x"),
                            Form::symbol("y"),
                        ]),
                        Form::Int(10),      // return 5
                    ]),
                    &env
                ),
                Ok(Value::Func(Lambda { params, .. })) if params == vec![SymbolId::from("x"), SymbolId::from("y")]
            ),
            "lambda special form returns a lambda value"
        );

        // Expect ((lambda (x) x) 5) => 5
        assert_eq!(
            eval(
                &Form::List(vec![
                    Form::List(vec![
                        Form::symbol("lambda"),
                        Form::List(vec![Form::symbol("x")]),
                        Form::symbol("x")
                    ]),
                    Form::Int(5)
                ]),
                &env
            ),
            Ok(Value::Int(5))
        );
    }
}
