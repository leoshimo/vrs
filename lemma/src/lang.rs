//! Defines opt-in language constructs
//! Lemma interpreter does not have built-in procedures and special forms by default.
//! The language features are "opt in" by defining symbols within the environment

use crate::{eval, Env, Error, Form, Lambda, Result, SpecialForm, SymbolId, Value};

/// Returns the 'standard' environment of the langugae
pub fn std_env<'a>() -> Env<'a> {
    let mut env = Env::new();
    add_lambda(&mut env);
    add_quote(&mut env);
    add_eval(&mut env);
    add_define(&mut env);
    add_if(&mut env);
    add_list(&mut env);
    add_push(&mut env);
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
            func: lang_eval,
        }),
    );
}

/// Adds the `define` symbol for defining values of symbols
pub fn add_define(env: &mut Env) {
    let define_sym = SymbolId::from("define");
    env.bind(
        &define_sym,
        Value::SpecialForm(SpecialForm {
            name: define_sym.to_string(),
            func: define,
        }),
    );
}

/// Adds the `if` symbol for conditional branching
pub fn add_if(env: &mut Env) {
    let if_sym = SymbolId::from("if");
    env.bind(
        &if_sym,
        Value::SpecialForm(SpecialForm {
            name: if_sym.to_string(),
            func: lang_if,
        }),
    );
}

/// Adds the `list` symbol for creating a list
pub fn add_list(env: &mut Env) {
    let sym = SymbolId::from("list");
    env.bind(
        &sym,
        Value::SpecialForm(SpecialForm {
            name: sym.to_string(),
            func: lang_list,
        }),
    );
}

/// Adds the `push` symbol for appending to list
pub fn add_push(env: &mut Env) {
    let sym = SymbolId::from("push");
    env.bind(
        &sym,
        Value::SpecialForm(SpecialForm {
            name: sym.to_string(),
            func: lang_push,
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
    Ok(Value::Lambda(Lambda { params, body }))
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
fn lang_eval(arg_forms: &[Form], env: &mut Env) -> Result<Value> {
    let arg_form = match arg_forms {
        [form] => Ok(form),
        _ => Err(Error::EvalExpectsSingleFormArgument),
    }?;

    match eval(arg_form, env)? {
        Value::Form(f) => eval(&f, env),
        _ => Err(Error::EvalExpectsSingleFormArgument),
    }
}

/// Implements the `define` special form
fn define(arg_forms: &[Form], env: &mut Env) -> Result<Value> {
    let (sym_id, val_form) = match arg_forms {
        [Form::Symbol(s), form] => Ok((s, form)),
        _ => Err(Error::UnexpectedArguments(
            "define expects a symbol and single form as arguments".to_string(),
        )),
    }?;

    let val = eval::eval(val_form, env)?;
    env.bind(sym_id, val.clone());
    Ok(val)
}

/// Implements the `if` condition
fn lang_if(arg_forms: &[Form], env: &mut Env) -> Result<Value> {
    let (cond_form, true_form, false_form) = match arg_forms {
        [cond_form, true_form, false_form] => Ok((cond_form, true_form, false_form)),
        _ => Err(Error::UnexpectedArguments(
            "if expects a condition form, true form, and false form".to_string(),
        )),
    }?;

    let cond = match eval(cond_form, env)? {
        Value::Form(Form::Bool(b)) => Ok(b),
        v => Err(Error::UnexpectedConditionalValue(format!(
            "Conditional form evaluated to {}",
            v
        ))),
    }?;

    if cond {
        eval(true_form, env)
    } else {
        eval(false_form, env)
    }
}

fn lang_list(arg_forms: &[Form], env: &mut Env) -> Result<Value> {
    let arg_vals = arg_forms
        .iter()
        .map(|f| eval(f, env))
        .collect::<Result<Vec<_>>>()?;
    Ok(Value::from(arg_vals))
}

/// Implements the `push` operation
fn lang_push(arg_forms: &[Form], env: &mut Env) -> Result<Value> {
    let (list_form, elem_form) = match arg_forms {
        [list_form, elem_form] => Ok((list_form, elem_form)),
        _ => Err(Error::UnexpectedArguments(format!(
            "push expects a two forms - got {}",
            arg_forms.len()
        ))),
    }?;

    let mut list_val = match eval(list_form, env)? {
        Value::List(l) => Ok(l),
        v => Err(Error::UnexpectedArguments(format!(
            "push expects first argument to evaluate to a list - got {}",
            v
        ))),
    }?;

    let elem_val = eval(elem_form, env)?;
    list_val.push(elem_val);
    Ok(Value::from(list_val))
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
    fn eval_define_vals() {
        {
            let mut env = std_env();
            assert_eq!(eval_expr("(define x 10)", &mut env), Ok(Value::from(10)));
        }

        {
            let mut env = std_env();
            assert_eq!(
                eval_expr("(define x \"hello\")", &mut env),
                Ok(Value::from("hello"))
            );
        }

        {
            // define + eval
            let mut env = std_env();
            assert_eq!(
                eval_expr("(define x \"hello\")", &mut env),
                Ok(Value::from("hello"))
            );

            assert_eq!(
                eval_expr("x", &mut env),
                Ok(Value::from("hello")),
                "x should evaluate to defined value"
            );
        }
    }

    #[test]
    #[traced_test]
    fn eval_define_func() {
        let mut env = std_env();

        assert!(matches!(
            eval_expr("(define echo (lambda (x) x))", &mut env),
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

        eval_expr("(define is_true true)", &mut env).unwrap();
        eval_expr("(define is_false false)", &mut env).unwrap();

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

        eval_expr("(define is_true (lambda () true))", &mut env).unwrap();
        eval_expr("(define is_false (lambda () false))", &mut env).unwrap();

        assert_eq!(
            eval_expr("(if (is_true) \"true\" \"false\")", &mut env),
            Ok(Value::from("true"))
        );

        assert_eq!(
            eval_expr("(if (is_false) \"true\" \"false\")", &mut env),
            Ok(Value::from("false"))
        );
    }

    #[test]
    #[traced_test]
    fn eval_list() {
        let mut env = std_env();

        assert_eq!(eval_expr("(list)", &mut env), Ok(Value::List(vec![])),);

        assert_eq!(
            eval_expr("(list 1 2 3)", &mut env),
            Ok(Value::List(vec![
                Value::from(1),
                Value::from(2),
                Value::from(3),
            ]))
        );

        assert_eq!(
            eval_expr("(list \"one\" 2 :three)", &mut env),
            Ok(Value::List(vec![
                Value::from("one"),
                Value::from(2),
                Value::from(Form::keyword("three")),
            ]))
        );

        eval_expr("(define echo (lambda (x) x))", &mut env).expect("Should define echo function");

        assert_eq!(
            eval_expr("(list (echo \"one\") (echo 2) (echo :three))", &mut env),
            Ok(Value::List(vec![
                Value::from("one"),
                Value::from(2),
                Value::from(Form::keyword("three")),
            ]))
        );
    }

    #[test]
    #[traced_test]
    fn eval_push() {
        let mut env = std_env();

        assert_eq!(
            eval_expr("(define my_list (list))", &mut env),
            Ok(<Value as From<Vec<Value>>>::from(vec![])),
            "should define a empty list"
        );

        assert_eq!(
            eval_expr("(push my_list 1)", &mut env),
            Ok(Value::from(vec![Value::from(1)])),
            "should return new list with new element"
        );

        assert_eq!(
            eval_expr("my_list", &mut env),
            Ok(<Value as From<Vec<Value>>>::from(vec![])),
            "original list should be unchanged"
        );

        assert_eq!(
            eval_expr("(define my_list (push my_list 1))", &mut env),
            Ok(Value::from(vec![Value::from(1)])),
        );

        assert_eq!(
            eval_expr("(define my_list (push my_list \"two\"))", &mut env),
            Ok(Value::from(vec![Value::from(1), Value::from("two"),])),
        );

        assert_eq!(
            eval_expr("(define my_list (push my_list :three))", &mut env),
            Ok(Value::from(vec![
                Value::from(1),
                Value::from("two"),
                Value::from(Form::keyword("three")),
            ])),
        );

        assert_eq!(
            eval_expr("my_list", &mut env),
            Ok(Value::from(vec![
                Value::from(1),
                Value::from("two"),
                Value::from(Form::keyword("three")),
            ])),
            "list should be mutated"
        );
    }
}
