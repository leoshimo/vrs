//! Defines opt-in language constructs
//! Lemma interpreter does not have built-in procedures and special forms by default.
//! The language features are "opt in" by defining symbols within the environment

use crate::{eval::eval, Env, Error, Form, Lambda, Result};

/// Implements `lambda` special form
pub fn lang_lambda(arg_forms: &[Form], _env: &mut Env) -> Result<Form> {
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
    Ok(Form::Lambda(Lambda { params, body }))
}

/// Implements the `quote` special form
pub fn lang_quote(arg_forms: &[Form], _env: &mut Env) -> Result<Form> {
    match arg_forms {
        [f] => Ok(f.clone()),
        _ => Err(Error::UnexpectedArguments(
            "quote expects one argument".to_string(),
        )),
    }
}

/// Implements the `eval` special form
pub fn lang_eval(arg_forms: &[Form], env: &mut Env) -> Result<Form> {
    let arg_form = match arg_forms {
        [form] => Ok(form),
        _ => Err(Error::UnexpectedArguments(
            "eval expects one argument".to_string(),
        )),
    }?;

    eval(&eval(arg_form, env)?, env)
}

/// Implements the `def` special form
pub fn lang_def(arg_forms: &[Form], env: &mut Env) -> Result<Form> {
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
pub fn lang_if(arg_forms: &[Form], env: &mut Env) -> Result<Form> {
    let (cond_form, true_form, false_form) = match arg_forms {
        [cond_form, true_form, false_form] => Ok((cond_form, true_form, false_form)),
        _ => Err(Error::UnexpectedArguments(
            "if expects a condition form, true form, and false form".to_string(),
        )),
    }?;

    let cond = match eval(cond_form, env)? {
        Form::Bool(b) => Ok(b),
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

/// Implements the `type` proc
pub fn lang_type(arg_forms: &[Form], env: &mut Env) -> Result<Form> {
    let f = match arg_forms {
        [f] => Ok(f),
        _ => Err(Error::UnexpectedArguments(
            "type expects one argument".to_string(),
        )),
    }?;

    Ok(match eval(&f, env)? {
        Form::Nil => Form::keyword("nil"),
        Form::Bool(_) => Form::keyword("bool"),
        Form::Int(_) => Form::keyword("int"),
        Form::String(_) => Form::keyword("string"),
        Form::Symbol(_) => Form::keyword("symbol"),
        Form::Keyword(_) => Form::keyword("keyword"),
        Form::List(_) => Form::keyword("list"),
        Form::Lambda(_) => Form::keyword("lambda"),
        Form::NativeFunc(_) => Form::keyword("nativefn"),
    })
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
            matches!(eval_expr("lambda", &mut env), Ok(Form::NativeFunc(_))),
            "lambda symbol should be defined"
        );

        assert!(
            matches!(
                eval_expr(
                    "(lambda (x y) 10)",
                    &mut env
                ),
                Ok(Form::Lambda(Lambda { params, .. })) if params == vec![SymbolId::from("x"), SymbolId::from("y")]
            ),
            "lambda special form returns a lambda value"
        );

        // ((lambda (x) x) 5) => 5
        assert_eq!(eval_expr("((lambda (x) x) 5)", &mut env), Ok(Form::Int(5)));

        // ((lambda () (lambda (x) x)))
        assert!(matches!(
            eval_expr("((lambda () (lambda (x) x)))", &mut env),
            Ok(Form::Lambda(_))
        ));

        // (((lambda () (lambda (x) x))) 10) => 10
        assert_eq!(
            eval_expr("(((lambda () (lambda (x) x))) 10)", &mut env),
            Ok(Form::Int(10))
        );
    }

    #[test]
    #[traced_test]
    fn eval_quote() {
        let mut env = std_env();

        assert_eq!(
            eval_expr("(quote (one :two three))", &mut env),
            Ok(Form::List(vec![
                Form::symbol("one"),
                Form::keyword("two"),
                Form::symbol("three"),
            ]))
        );

        assert_eq!(
            eval_expr("(quote (lambda (x) x))", &mut env),
            Ok(Form::List(vec![
                Form::symbol("lambda"),
                Form::List(vec![Form::symbol("x")]),
                Form::symbol("x"),
            ]))
        );

        assert!(
            matches!(
                eval_expr("((quote (lambda (x) x)) 5)", &mut env),
                Err(Error::NotAProcedure(_))
            ),
            "A quoted operation does not recursively evaluate without explicit call to eval"
        );
    }

    #[test]
    #[traced_test]
    fn eval_eval() {
        let mut env = std_env();

        assert_eq!(eval_expr("(eval (quote 5))", &mut env), Ok(Form::Int(5)));

        assert_eq!(
            eval_expr("(eval (quote ((lambda (x) x) 5)))", &mut env),
            Ok(Form::Int(5))
        );
    }

    #[test]
    #[traced_test]
    fn eval_def_vals() {
        {
            let mut env = std_env();
            assert_eq!(eval_expr("(def x 10)", &mut env), Ok(Form::Int(10)));
        }

        {
            let mut env = std_env();
            assert_eq!(
                eval_expr("(def x \"hello\")", &mut env),
                Ok(Form::string("hello"))
            );
        }

        {
            // def + eval
            let mut env = std_env();
            assert_eq!(
                eval_expr("(def x \"hello\")", &mut env),
                Ok(Form::string("hello"))
            );

            assert_eq!(
                eval_expr("x", &mut env),
                Ok(Form::string("hello")),
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
            Ok(Form::Lambda(_))
        ));

        assert_eq!(
            eval_expr("(echo \"hello\")", &mut env),
            Ok(Form::string("hello"))
        );

        assert_eq!(
            eval_expr("(echo (echo \"hello\"))", &mut env),
            Ok(Form::string("hello"))
        );
    }

    #[test]
    #[traced_test]
    fn eval_if() {
        let mut env = std_env();

        assert_eq!(
            eval_expr("(if true \"true\" \"false\")", &mut env),
            Ok(Form::string("true"))
        );

        assert_eq!(
            eval_expr("(if false \"true\" \"false\")", &mut env),
            Ok(Form::string("false"))
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
            Ok(Form::string("true"))
        );

        assert_eq!(
            eval_expr("(if is_false \"true\" \"false\")", &mut env),
            Ok(Form::string("false"))
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
            Ok(Form::string("true"))
        );

        assert_eq!(
            eval_expr("(if (is_false) \"true\" \"false\")", &mut env),
            Ok(Form::string("false"))
        );
    }

    #[test]
    #[traced_test]
    fn eval_type() {
        let mut env = std_env();

        assert_eq!(eval_expr("(type nil)", &mut env), Ok(Form::keyword("nil")));
        assert_eq!(
            eval_expr("(type true)", &mut env),
            Ok(Form::keyword("bool"))
        );
        assert_eq!(
            eval_expr("(type false)", &mut env),
            Ok(Form::keyword("bool"))
        );
        assert_eq!(eval_expr("(type 1)", &mut env), Ok(Form::keyword("int")));
        assert_eq!(
            eval_expr("(type \"one\")", &mut env),
            Ok(Form::keyword("string"))
        );
        assert_eq!(
            eval_expr("(type :a_keyword)", &mut env),
            Ok(Form::keyword("keyword"))
        );
        assert_eq!(
            eval_expr("(type (quote ()))", &mut env),
            Ok(Form::keyword("list"))
        );
        assert_eq!(
            eval_expr("(type (lambda (x) x))", &mut env),
            Ok(Form::keyword("lambda"))
        );
        assert_eq!(
            eval_expr("(type type)", &mut env),
            Ok(Form::keyword("nativefn"))
        );
        assert_eq!(
            eval_expr("(type ((lambda (x) x) 5))", &mut env),
            Ok(Form::keyword("int"))
        );
    }
}
