//! List Related Special Forms
use crate::eval::{eval, eval_lambda_call_vals};
use crate::{Env, Error, Form, Result, Value};

/// Implements the `list` binding
pub fn lang_list(arg_forms: &[Form], env: &mut Env) -> Result<Value> {
    let arg_vals = arg_forms
        .iter()
        .map(|f| eval(f, env))
        .collect::<Result<Vec<_>>>()?;
    Ok(Value::from(arg_vals))
}

/// Implements the `len` binding
pub fn lang_len(arg_forms: &[Form], env: &mut Env) -> Result<Value> {
    let f = match arg_forms {
        [f] => Ok(f),
        _ => Err(Error::UnexpectedArguments(
            "length expects one argument".to_string(),
        )),
    }?;

    let length = match eval(f, env)? {
        Value::Form(Form::List(l)) => Ok(l.len()),
        Value::List(v) => Ok(v.len()),
        _ => Err(Error::UnexpectedArguments(
            "length expects a collection type".to_string(),
        )),
    }?;

    Ok(Value::from(length as i32))
}

/// Implements the `get` binding
pub fn lang_get(arg_forms: &[Form], env: &mut Env) -> Result<Value> {
    let (coll_form, spec_form) = match arg_forms {
        [coll_form, spec_form] => Ok((coll_form, spec_form)),
        _ => Err(Error::UnexpectedArguments(
            "get expects two arguments".to_string(),
        )),
    }?;

    let values = match eval(coll_form, env)? {
        Value::Form(Form::List(l)) => Ok(l.into_iter().map(Value::from).collect::<Vec<_>>()),
        Value::List(v) => Ok(v),
        _ => Err(Error::UnexpectedArguments(
            "get expects a collection type".to_string(),
        )),
    }?;

    match eval(spec_form, env)? {
        Value::Form(Form::Int(idx)) => match values.get(idx as usize) {
            Some(v) => Ok(v.clone()),
            None => Ok(Value::from(Form::Nil)),
        },
        _ => Err(Error::UnexpectedArguments(
            "get expects integer as index".to_string(),
        )),
    }
}
/// Implements the `getn` binding
pub fn lang_getn(arg_forms: &[Form], env: &mut Env) -> Result<Value> {
    let (coll_form, target_form) = match arg_forms {
        [c, t] => Ok((c, t)),
        _ => Err(Error::UnexpectedArguments(
            "getn expects two arguments".to_string(),
        )),
    }?;

    let values = match eval(coll_form, env)? {
        Value::Form(Form::List(l)) => Ok(l.into_iter().map(Value::from).collect::<Vec<_>>()),
        Value::List(v) => Ok(v),
        _ => Err(Error::UnexpectedArguments(
            "getn expects a collection type".to_string(),
        )),
    }?;

    let target_val = eval(target_form, env)?;
    let mut iter = values.into_iter().skip_while(|v| v != &target_val);
    let _ = iter.next();
    match iter.next() {
        Some(v) => Ok(v),
        None => Ok(Value::from(Form::Nil)),
    }
}

/// Implements the `map` binding
pub fn lang_map(arg_forms: &[Form], env: &mut Env) -> Result<Value> {
    let (coll_form, lambda_form) = match arg_forms {
        [coll_form, lambda_form] => Ok((coll_form, lambda_form)),
        _ => Err(Error::UnexpectedArguments(format!(
            "map expects a two forms - got {}",
            arg_forms.len()
        ))),
    }?;

    let vals = match eval(coll_form, env)? {
        Value::Form(Form::List(forms)) => Ok(forms.into_iter().map(Value::from).collect()),
        Value::List(v) => Ok(v),
        v => Err(Error::UnexpectedArguments(format!(
            "map expects first argument to be a collection. Got {}",
            v
        ))),
    }?;

    let lambda = match eval(lambda_form, env)? {
        Value::Lambda(l) => Ok(l),
        _ => Err(Error::UnexpectedArguments(
            "map expects second argument to be a lambda".to_string(),
        )),
    }?;

    let res = vals
        .into_iter()
        .map(|v| eval_lambda_call_vals(&lambda, &[v], env))
        .collect::<Result<Vec<_>>>()?;

    Ok(Value::List(res))
}

/// Implements the `push` operation on list
pub fn lang_push(arg_forms: &[Form], env: &mut Env) -> Result<Value> {
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

/// Implements the `pushd` operation on list
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

#[cfg(test)]
mod tests {

    use super::*;
    use crate::eval_expr;
    use crate::lang::std_env;
    use tracing_test::traced_test;

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

        eval_expr("(def echo (lambda (x) x))", &mut env).expect("Should define echo function");

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
    fn eval_len() {
        let mut env = std_env();

        assert!(matches!(
            eval_expr("(len)", &mut env),
            Err(Error::UnexpectedArguments(_))
        ));

        assert!(matches!(
            eval_expr("(len 0)", &mut env),
            Err(Error::UnexpectedArguments(_))
        ));

        assert_eq!(eval_expr("(len (quote ()))", &mut env), Ok(Value::from(0)));

        assert_eq!(
            eval_expr("(len (quote (1 2 3 4 5)))", &mut env),
            Ok(Value::from(5))
        );

        assert_eq!(
            eval_expr("(len (list :one 2 \"three\"))", &mut env),
            Ok(Value::from(3))
        );
    }

    #[test]
    #[traced_test]
    fn eval_get() {
        let mut env = std_env();

        assert!(matches!(
            eval_expr("(get \"hello\" 0)", &mut env),
            Err(Error::UnexpectedArguments(_))
        ));

        assert_eq!(
            eval_expr("(get (quote ()) 0)", &mut env),
            Ok(Value::from(Form::Nil))
        );

        assert_eq!(
            eval_expr("(get (quote (1 2 3)) 0)", &mut env),
            Ok(Value::from(1)),
        );

        assert_eq!(
            eval_expr("(get (list :one :two :three) 2)", &mut env),
            Ok(Value::from(Form::keyword("three")))
        );
    }

    #[test]
    #[traced_test]
    fn eval_getn() {
        let mut env = std_env();

        assert!(matches!(
            eval_expr("(getn \"hello\" 0)", &mut env),
            Err(Error::UnexpectedArguments(_))
        ));

        assert_eq!(
            eval_expr("(getn (quote ()) 0)", &mut env),
            Ok(Value::from(Form::Nil))
        );

        assert_eq!(
            eval_expr("(getn (quote (1 2 3)) 1)", &mut env),
            Ok(Value::from(2)),
        );

        assert_eq!(
            eval_expr(
                "(getn (list :one \"one\" :two \"two\" :three \"three\") :two)",
                &mut env
            ),
            Ok(Value::from(Form::string("two")))
        );
    }

    #[test]
    #[traced_test]
    fn eval_map() {
        let mut env = std_env();

        eval_expr("(def echo (lambda (x) x))", &mut env).unwrap();
        eval_expr("(def zero (lambda (x) 0))", &mut env).unwrap();

        assert_eq!(
            eval_expr("(map (quote ()) (lambda (x) x))", &mut env),
            Ok(Value::List(vec![]))
        );

        assert_eq!(
            eval_expr("(map (quote (:one \"two\" 3)) echo)", &mut env),
            Ok(Value::List(vec![
                Value::from(Form::keyword("one")),
                Value::from("two"),
                Value::from(3),
            ]))
        );

        assert_eq!(
            eval_expr("(map (quote (1 2 3)) echo)", &mut env),
            Ok(Value::List(vec![
                Value::from(1),
                Value::from(2),
                Value::from(3),
            ]))
        );

        assert_eq!(
            eval_expr("(map (quote (1 2 3)) zero)", &mut env),
            Ok(Value::List(vec![
                Value::from(0),
                Value::from(0),
                Value::from(0),
            ]))
        );
    }

    #[test]
    #[traced_test]
    fn eval_push() {
        let mut env = std_env();

        assert_eq!(
            eval_expr("(def my_lst (list))", &mut env),
            Ok(<Value as From<Vec<Value>>>::from(vec![])),
            "should define a empty list"
        );

        assert_eq!(
            eval_expr("(push my_lst 1)", &mut env),
            Ok(Value::from(vec![Value::from(1)])),
            "should return new list with new element"
        );

        assert_eq!(
            eval_expr("my_lst", &mut env),
            Ok(<Value as From<Vec<Value>>>::from(vec![])),
            "original list should be unchanged"
        );

        assert_eq!(
            eval_expr("(def my_lst (push my_lst 1))", &mut env),
            Ok(Value::from(vec![Value::from(1)])),
        );

        assert_eq!(
            eval_expr("(def my_lst (push my_lst \"two\"))", &mut env),
            Ok(Value::from(vec![Value::from(1), Value::from("two"),])),
        );

        assert_eq!(
            eval_expr("(def my_lst (push my_lst :three))", &mut env),
            Ok(Value::from(vec![
                Value::from(1),
                Value::from("two"),
                Value::from(Form::keyword("three")),
            ])),
        );

        assert_eq!(
            eval_expr("my_lst", &mut env),
            Ok(Value::from(vec![
                Value::from(1),
                Value::from("two"),
                Value::from(Form::keyword("three")),
            ])),
            "list should be mutated"
        );
    }

    #[test]
    #[traced_test]
    fn eval_pushd() {
        let mut env = std_env();

        eval_expr("(def my_lst (list))", &mut env).unwrap();

        assert_eq!(
            eval_expr("(pushd my_lst 1)", &mut env),
            Ok(Value::from(vec![Value::from(1)])),
        );

        assert_eq!(
            eval_expr("my_lst", &mut env),
            Ok(Value::from(vec![Value::from(1),])),
        );

        assert_eq!(
            eval_expr("(pushd my_lst :two)", &mut env),
            Ok(Value::from(vec![
                Value::from(1),
                Value::from(Form::keyword("two")),
            ])),
        );

        assert_eq!(
            eval_expr("(pushd my_lst \"three\")", &mut env),
            Ok(Value::from(vec![
                Value::from(1),
                Value::from(Form::keyword("two")),
                Value::from("three"),
            ])),
        );

        assert_eq!(
            eval_expr("my_lst", &mut env),
            Ok(Value::from(vec![
                Value::from(1),
                Value::from(Form::keyword("two")),
                Value::from("three"),
            ])),
        )
    }
}
