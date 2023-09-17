//! Defines opt-in language constructs
//! Lemma interpreter does not have built-in procedures and special forms by default.
//! The language features are "opt in" by defining symbols within the environment

use crate::{
    eval::{eval, eval_lambda_call_vals},
    Env, Error, Form, Lambda, Result, Value,
};

/// Implements `lambda` special form
pub fn lang_lambda(arg_forms: &[Form], _env: &mut Env) -> Result<Value> {
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
pub fn lang_quote(arg_forms: &[Form], _env: &mut Env) -> Result<Value> {
    if arg_forms.len() == 1 {
        Ok(Value::Form(arg_forms[0].clone()))
    } else {
        Err(Error::QuoteExpectsSingleArgument)
    }
}

/// Implements the `eval` special form
pub fn lang_eval(arg_forms: &[Form], env: &mut Env) -> Result<Value> {
    let arg_form = match arg_forms {
        [form] => Ok(form),
        _ => Err(Error::EvalExpectsSingleFormArgument),
    }?;

    match eval(arg_form, env)? {
        Value::Form(f) => eval(&f, env),
        _ => Err(Error::EvalExpectsSingleFormArgument),
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

pub fn lang_vec(arg_forms: &[Form], env: &mut Env) -> Result<Value> {
    let arg_vals = arg_forms
        .iter()
        .map(|f| eval(f, env))
        .collect::<Result<Vec<_>>>()?;
    Ok(Value::from(arg_vals))
}

pub fn lang_length(arg_forms: &[Form], env: &mut Env) -> Result<Value> {
    let f = match arg_forms {
        [f] => Ok(f),
        _ => Err(Error::UnexpectedArguments(
            "length expects one argument".to_string(),
        )),
    }?;

    let length = match eval(f, env)? {
        Value::Form(Form::List(l)) => Ok(l.len()),
        Value::Vec(v) => Ok(v.len()),
        _ => Err(Error::UnexpectedArguments(
            "length expects a collection type".to_string(),
        )),
    }?;

    Ok(Value::from(length as i32))
}

pub fn lang_get(arg_forms: &[Form], env: &mut Env) -> Result<Value> {
    let (coll_form, spec_form) = match arg_forms {
        [coll_form, spec_form] => Ok((coll_form, spec_form)),
        _ => Err(Error::UnexpectedArguments(
            "get expects two arguments".to_string(),
        )),
    }?;

    let values = match eval(coll_form, env)? {
        Value::Form(Form::List(l)) => Ok(l.into_iter().map(Value::from).collect::<Vec<_>>()),
        Value::Vec(v) => Ok(v),
        _ => Err(Error::UnexpectedArguments(
            "length expects a collection type".to_string(),
        )),
    }?;

    match eval(spec_form, env)? {
        Value::Form(Form::Keyword(keyword)) => {
            // get by sym
            let next_element: Option<Value> = values
                .windows(2)
                .filter_map(|w| {
                    if w[0] == Value::Form(Form::Keyword(keyword.clone())) {
                        w.get(1)
                    } else {
                        None
                    }
                })
                .next()
                .cloned();
            match next_element {
                Some(v) => Ok(v),
                None => Err(Error::UnexpectedArguments(
                    "no element with matching keyword".to_string(),
                )),
            }
        }
        Value::Form(Form::Int(idx)) => {
            // get by idx
            match values.get(idx as usize) {
                Some(v) => Ok(v.clone()),
                None => Err(Error::UnexpectedArguments(
                    "no element at index".to_string(),
                )),
            }
        }
        _ => Err(Error::UnexpectedArguments(
            "Unsupported get spec".to_string(),
        )),
    }
}

pub fn lang_map(arg_forms: &[Form], env: &mut Env) -> Result<Value> {
    let (coll_form, lambda_form) = match arg_forms {
        [coll_form, lambda_form] => Ok((coll_form, lambda_form)),
        _ => Err(Error::UnexpectedArguments(format!(
            "map expects a two forms - got {}",
            arg_forms.len()
        ))),
    }?;

    // TODO Think about this case - evaluation yields a Value, but value is passed into lambda
    let vals = match eval(coll_form, env)? {
        Value::Vec(v) => Ok(v),
        _ => Err(Error::UnexpectedArguments(
            "map expects first argument to be a collection".to_string(),
        )),
    }?;

    let lambda = match eval(lambda_form, env)? {
        Value::Lambda(l) => Ok(l),
        _ => Err(Error::UnexpectedArguments(
            "map expects second argument to be a lambda".to_string(),
        )),
    }?;

    // This is similar to `eval_lambda_call` but also a little bit diff.
    let mut res = vec![];
    for v in vals {
        res.push(eval_lambda_call_vals(&lambda, &[v], env)?);
    }

    Ok(Value::Vec(res))
}

/// Implements the `push` operation on vector
pub fn lang_push(arg_forms: &[Form], env: &mut Env) -> Result<Value> {
    let (vec_form, elem_form) = match arg_forms {
        [vec_form, elem_form] => Ok((vec_form, elem_form)),
        _ => Err(Error::UnexpectedArguments(format!(
            "push expects a two forms - got {}",
            arg_forms.len()
        ))),
    }?;

    let mut vec_val = match eval(vec_form, env)? {
        Value::Vec(l) => Ok(l),
        v => Err(Error::UnexpectedArguments(format!(
            "push expects first argument to evaluate to a vector - got {}",
            v
        ))),
    }?;

    let elem_val = eval(elem_form, env)?;
    vec_val.push(elem_val);
    Ok(Value::from(vec_val))
}

/// Implements the `pushd` operation on vector
pub fn lang_pushd(arg_forms: &[Form], env: &mut Env) -> Result<Value> {
    let (symbol, elem_form) = match arg_forms {
        [Form::Symbol(symbol), elem_form] => Ok((symbol.clone(), elem_form.clone())),
        _ => Err(Error::UnexpectedArguments(
            "pushd expects a one symbol form and one value form".to_string(),
        )),
    }?;

    let val = lang_push(&[Form::Symbol(symbol.clone()), elem_form], env)?;
    env.bind(&symbol, val.clone());
    Ok(val)
}

/// Implements the `as_form` operation on vector
pub fn lang_as_form(arg_forms: &[Form], env: &mut Env) -> Result<Value> {
    let form = match arg_forms {
        [f] => Ok(f),
        _ => Err(Error::UnexpectedArguments(format!(
            "as_form expects a single forms - got {}",
            arg_forms.len()
        ))),
    }?;

    let val = eval(form, env)?;
    let coerced_val =
        Form::try_from(val).map_err(|e| Error::UnsupportedValueConversion(e.to_string()))?;
    Ok(Value::from(coerced_val))
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

    #[test]
    #[traced_test]
    fn eval_vec() {
        let mut env = std_env();

        assert_eq!(eval_expr("(vec)", &mut env), Ok(Value::Vec(vec![])),);

        assert_eq!(
            eval_expr("(vec 1 2 3)", &mut env),
            Ok(Value::Vec(vec![
                Value::from(1),
                Value::from(2),
                Value::from(3),
            ]))
        );

        assert_eq!(
            eval_expr("(vec \"one\" 2 :three)", &mut env),
            Ok(Value::Vec(vec![
                Value::from("one"),
                Value::from(2),
                Value::from(Form::keyword("three")),
            ]))
        );

        eval_expr("(def echo (lambda (x) x))", &mut env).expect("Should define echo function");

        assert_eq!(
            eval_expr("(vec (echo \"one\") (echo 2) (echo :three))", &mut env),
            Ok(Value::Vec(vec![
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
            eval_expr("(def my_vec (vec))", &mut env),
            Ok(<Value as From<Vec<Value>>>::from(vec![])),
            "should define a empty vec"
        );

        assert_eq!(
            eval_expr("(push my_vec 1)", &mut env),
            Ok(Value::from(vec![Value::from(1)])),
            "should return new vec with new element"
        );

        assert_eq!(
            eval_expr("my_vec", &mut env),
            Ok(<Value as From<Vec<Value>>>::from(vec![])),
            "original vec should be unchanged"
        );

        assert_eq!(
            eval_expr("(def my_vec (push my_vec 1))", &mut env),
            Ok(Value::from(vec![Value::from(1)])),
        );

        assert_eq!(
            eval_expr("(def my_vec (push my_vec \"two\"))", &mut env),
            Ok(Value::from(vec![Value::from(1), Value::from("two"),])),
        );

        assert_eq!(
            eval_expr("(def my_vec (push my_vec :three))", &mut env),
            Ok(Value::from(vec![
                Value::from(1),
                Value::from("two"),
                Value::from(Form::keyword("three")),
            ])),
        );

        assert_eq!(
            eval_expr("my_vec", &mut env),
            Ok(Value::from(vec![
                Value::from(1),
                Value::from("two"),
                Value::from(Form::keyword("three")),
            ])),
            "vec should be mutated"
        );
    }
}
