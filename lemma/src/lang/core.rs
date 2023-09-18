//! Defines opt-in language constructs
//! Lemma interpreter does not have built-in procedures and special forms by default.
//! The language features are "opt in" by defining symbols within the environment

use crate::{eval::eval, Env, Error, Form, Lambda, Result, Value};

/// Implements `lambda` special form
pub fn lang_lambda(arg_forms: &[Form], _env: &mut Env) -> Result<Value> {
    let (params, body) = arg_forms.split_first().ok_or(Error::UnexpectedArguments(
        "lambda expects two arguments".to_string(),
    ))?;

    let params = match params {
        Form::List(l) => Ok(l),
        _ => Err(Error::UnexpectedArguments(
            "first argument should be a list".to_string(),
        )),
    }?;

    let params = params
        .iter()
        .map(|p| match p {
            Form::Symbol(s) => Ok(s.clone()),
            _ => Err(Error::UnexpectedArguments(format!(
                "invalid element in parameter list - {}",
                p
            ))),
        })
        .collect::<Result<Vec<_>>>()?;

    let body = body.to_owned();
    Ok(Value::Lambda(Lambda { params, body }))
}

/// Implements the `quote` special form
pub fn lang_quote(arg_forms: &[Form], _env: &mut Env) -> Result<Value> {
    match arg_forms {
        [f] => Ok(Value::Form(f.clone())),
        _ => Err(Error::UnexpectedArguments(
            "quote expects one argument".to_string(),
        )),
    }
}

/// Implements the `eval` special form
pub fn lang_eval(arg_forms: &[Form], env: &mut Env) -> Result<Value> {
    let arg_form = match arg_forms {
        [form] => Ok(form),
        _ => Err(Error::UnexpectedArguments(
            "eval expects one argument".to_string(),
        )),
    }?;

    match eval(arg_form, env)? {
        Value::Form(f) => eval(&f, env),
        _ => Err(Error::UnexpectedArguments(
            "eval expects form as argument".to_string(),
        )),
    }
}

/// Implements the `def` special form
pub fn lang_def(arg_forms: &[Form], env: &mut Env) -> Result<Value> {
    let (sym_id, val_form) = match arg_forms {
        [Form::Symbol(s), form] => Ok((s, form)),
        _ => Err(Error::UnexpectedArguments(
            "def expects a symbol and single form as arguments".to_string(),
        )),
    }?;

    let val = eval(val_form, env)?;
    env.bind(sym_id, val.clone());
    Ok(val)
}

/// Implements the `if` condition
pub fn lang_if(arg_forms: &[Form], env: &mut Env) -> Result<Value> {
    let (cond_form, true_form, false_form) = match arg_forms {
        [cond_form, true_form, false_form] => Ok((cond_form, true_form, false_form)),
        _ => Err(Error::UnexpectedArguments(
            "if expects a condition form, true form, and false form".to_string(),
        )),
    }?;

    let cond = match eval(cond_form, env)? {
        Value::Form(Form::Bool(b)) => Ok(b),
        v => Err(Error::UnexpectedArguments(format!(
            "conditional form evaluated to {}",
            v
        ))),
    }?;

    if cond {
        eval(true_form, env)
    } else {
        eval(false_form, env)
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::lang::std_env;
    use crate::{eval_expr, SymbolId};
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
                Ok(Value::Lambda(Lambda { params, .. })) if params == vec![SymbolId::from("x"), SymbolId::from("y")]
            ),
            "lambda special form returns a lambda value"
        );

        // ((lambda (x) x) 5) => 5
        assert_eq!(
            eval_expr("((lambda (x) x) 5)", &mut env),
            Ok(Value::from(5))
        );

        // ((lambda () (lambda (x) x))) => Value::Func
        assert!(matches!(
            eval_expr("((lambda () (lambda (x) x)))", &mut env),
            Ok(Value::Lambda(_))
        ));

        // (((lambda () (lambda (x) x))) 10) => 10
        assert_eq!(
            eval_expr("(((lambda () (lambda (x) x))) 10)", &mut env),
            Ok(Value::from(10))
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

        assert_eq!(eval_expr("(eval (quote 5))", &mut env), Ok(Value::from(5)));

        assert_eq!(
            eval_expr("(eval (quote ((lambda (x) x) 5)))", &mut env),
            Ok(Value::from(5))
        );
    }

    #[test]
    #[traced_test]
    fn eval_def_vals() {
        {
            let mut env = std_env();
            assert_eq!(eval_expr("(def x 10)", &mut env), Ok(Value::from(10)));
        }

        {
            let mut env = std_env();
            assert_eq!(
                eval_expr("(def x \"hello\")", &mut env),
                Ok(Value::from("hello"))
            );
        }

        {
            // def + eval
            let mut env = std_env();
            assert_eq!(
                eval_expr("(def x \"hello\")", &mut env),
                Ok(Value::from("hello"))
            );

            assert_eq!(
                eval_expr("x", &mut env),
                Ok(Value::from("hello")),
                "x should evaluate to def value"
            );
        }
    }

    #[test]
    #[traced_test]
    fn eval_def_func() {
        let mut env = std_env();

        assert!(matches!(
            eval_expr("(def echo (lambda (x) x))", &mut env),
            Ok(Value::Lambda(_))
        ));

        assert_eq!(
            eval_expr("(echo \"hello\")", &mut env),
            Ok(Value::from("hello"))
        );

        assert_eq!(
            eval_expr("(echo (echo \"hello\"))", &mut env),
            Ok(Value::from("hello"))
        );
    }

    #[test]
    #[traced_test]
    fn eval_if() {
        let mut env = std_env();

        assert_eq!(
            eval_expr("(if true \"true\" \"false\")", &mut env),
            Ok(Value::from("true"))
        );

        assert_eq!(
            eval_expr("(if false \"true\" \"false\")", &mut env),
            Ok(Value::from("false"))
        );
    }

    #[test]
    #[traced_test]
    fn eval_if_with_symbols() {
        let mut env = std_env();

        eval_expr("(def is_true true)", &mut env).unwrap();
        eval_expr("(def is_false false)", &mut env).unwrap();

        assert_eq!(
            eval_expr("(if is_true \"true\" \"false\")", &mut env),
            Ok(Value::from("true"))
        );

        assert_eq!(
            eval_expr("(if is_false \"true\" \"false\")", &mut env),
            Ok(Value::from("false"))
        );
    }

    #[test]
    #[traced_test]
    fn eval_if_with_lambda() {
        let mut env = std_env();

        eval_expr("(def is_true (lambda () true))", &mut env).unwrap();
        eval_expr("(def is_false (lambda () false))", &mut env).unwrap();

        assert_eq!(
            eval_expr("(if (is_true) \"true\" \"false\")", &mut env),
            Ok(Value::from("true"))
        );

        assert_eq!(
            eval_expr("(if (is_false) \"true\" \"false\")", &mut env),
            Ok(Value::from("false"))
        );
    }
}
