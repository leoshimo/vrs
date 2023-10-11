//! List Related Special Forms
use crate::eval::{eval, eval_lambda_call_vals};
use crate::{Env, Error, Form, Result};

/// Implements the `list` binding
pub fn lang_list(arg_forms: &[Form], env: &mut Env) -> Result<Form> {
    let arg_vals = arg_forms
        .iter()
        .map(|f| eval(f, env))
        .collect::<Result<Vec<_>>>()?;
    Ok(Form::List(arg_vals))
}

/// Implements the `len` binding
pub fn lang_len(arg_forms: &[Form], env: &mut Env) -> Result<Form> {
    let f = match arg_forms {
        [f] => Ok(f),
        _ => Err(Error::UnexpectedArguments(
            "length expects one argument".to_string(),
        )),
    }?;

    let length = match eval(f, env)? {
        Form::List(l) => Ok(l.len()),
        _ => Err(Error::UnexpectedArguments(
            "length expects a list type".to_string(),
        )),
    }?;

    Ok(Form::Int(length as i32))
}

/// Implements the `get` binding
pub fn lang_get(arg_forms: &[Form], env: &mut Env) -> Result<Form> {
    let (coll_form, spec_form) = match arg_forms {
        [coll_form, spec_form] => Ok((coll_form, spec_form)),
        _ => Err(Error::UnexpectedArguments(
            "get expects two arguments".to_string(),
        )),
    }?;

    let values = match eval(coll_form, env)? {
        Form::List(l) => Ok(l.clone()),
        _ => Err(Error::UnexpectedArguments(
            "get expects a list type".to_string(),
        )),
    }?;

    match eval(spec_form, env)? {
        Form::Int(idx) => match values.get(idx as usize) {
            Some(v) => Ok(v.clone()),
            None => Ok(Form::Nil),
        },
        _ => Err(Error::UnexpectedArguments(
            "get expects integer as index".to_string(),
        )),
    }
}
/// Implements the `getn` binding
pub fn lang_getn(arg_forms: &[Form], env: &mut Env) -> Result<Form> {
    let (coll_form, target_form) = match arg_forms {
        [c, t] => Ok((c, t)),
        _ => Err(Error::UnexpectedArguments(
            "getn expects two arguments".to_string(),
        )),
    }?;

    let values = match eval(coll_form, env)? {
        Form::List(l) => Ok(l.clone()),
        _ => Err(Error::UnexpectedArguments(
            "getn expects a list type".to_string(),
        )),
    }?;

    let target_val = eval(target_form, env)?;
    let mut iter = values.into_iter().skip_while(|v| v != &target_val);
    let _ = iter.next();
    match iter.next() {
        Some(v) => Ok(v),
        None => Ok(Form::Nil),
    }
}

/// Implements the `map` binding
pub fn lang_map(arg_forms: &[Form], env: &mut Env) -> Result<Form> {
    let (coll_form, lambda_form) = match arg_forms {
        [coll_form, lambda_form] => Ok((coll_form, lambda_form)),
        _ => Err(Error::UnexpectedArguments(format!(
            "map expects a two forms - got {}",
            arg_forms.len()
        ))),
    }?;

    let vals = match eval(coll_form, env)? {
        Form::List(l) => Ok(l.clone()),
        v => Err(Error::UnexpectedArguments(format!(
            "map expects first argument to be a list. Got {}",
            v
        ))),
    }?;

    let lambda = match eval(lambda_form, env)? {
        Form::Lambda(l) => Ok(l),
        _ => Err(Error::UnexpectedArguments(
            "map expects second argument to be a lambda".to_string(),
        )),
    }?;

    let res = vals
        .into_iter()
        .map(|v| eval_lambda_call_vals(&lambda, &[v], env))
        .collect::<Result<Vec<_>>>()?;

    Ok(Form::List(res))
}

/// Implements the `push` operation on list
pub fn lang_push(arg_forms: &[Form], env: &mut Env) -> Result<Form> {
    let (symbol, elem_form) = match arg_forms {
        [Form::Symbol(s), e] => Ok((s, e.clone())),
        _ => Err(Error::UnexpectedArguments(
            "push expects a one symbol form and one value form".to_string(),
        )),
    }?;

    let mut list_val = match env.resolve(symbol) {
        Some(Form::List(l)) => Ok(l.clone()),
        _ => Err(Error::UnexpectedArguments(
            "push expects first argument to evaluate to a list".to_string(),
        )),
    }?;

    let elem_val = eval(&elem_form, env)?;
    list_val.push(elem_val);
    env.bind(symbol, Form::List(list_val.clone()));
    Ok(Form::List(list_val))
}

/// Implements the `pop` operation on list
pub fn lang_pop(arg_forms: &[Form], env: &mut Env) -> Result<Form> {
    let symbol = match arg_forms {
        [Form::Symbol(s)] => Ok(s),
        _ => Err(Error::UnexpectedArguments(
            "pop expects one symbol form as argument".to_string(),
        )),
    }?;

    let mut list_val = match env.resolve(symbol) {
        Some(Form::List(l)) => Ok(l.clone()),
        _ => Err(Error::UnexpectedArguments(
            "pop expects first argument to evaluate to list".to_string(),
        )),
    }?;

    let val = list_val.pop();
    env.bind(symbol, Form::List(list_val.clone()));
    match val {
        Some(v) => Ok(v),
        None => Ok(Form::Nil),
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::eval_expr;
    use crate::lang::std_env;
    use tracing_test::traced_test;

    #[test]
    #[traced_test]
    #[ignore]
    fn eval_list() {
        let mut env = std_env();

        assert_eq!(eval_expr("(list)", &mut env), Ok(Form::List(vec![])),);

        assert_eq!(
            eval_expr("(list 1 2 3)", &mut env),
            Ok(Form::List(vec![Form::Int(1), Form::Int(2), Form::Int(3),]))
        );

        assert_eq!(
            eval_expr("(list \"one\" 2 :three)", &mut env),
            Ok(Form::List(vec![
                Form::string("one"),
                Form::Int(2),
                Form::keyword("three"),
            ]))
        );

        eval_expr("(def echo (lambda (x) x))", &mut env).expect("Should define echo function");

        assert_eq!(
            eval_expr("(list (echo \"one\") (echo 2) (echo :three))", &mut env),
            Ok(Form::List(vec![
                Form::string("one"),
                Form::Int(2),
                Form::keyword("three"),
            ]))
        );
    }

    #[test]
    #[traced_test]
    #[ignore]
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

        assert_eq!(eval_expr("(len (quote ()))", &mut env), Ok(Form::Int(0)));

        assert_eq!(
            eval_expr("(len (quote (1 2 3 4 5)))", &mut env),
            Ok(Form::Int(5))
        );

        assert_eq!(
            eval_expr("(len (list :one 2 \"three\"))", &mut env),
            Ok(Form::Int(3))
        );
    }

    #[test]
    #[traced_test]
    #[ignore]
    fn eval_get() {
        let mut env = std_env();

        assert!(matches!(
            eval_expr("(get \"hello\" 0)", &mut env),
            Err(Error::UnexpectedArguments(_))
        ));

        assert_eq!(eval_expr("(get (quote ()) 0)", &mut env), Ok(Form::Nil));

        assert_eq!(
            eval_expr("(get (quote (1 2 3)) 0)", &mut env),
            Ok(Form::Int(1)),
        );

        assert_eq!(
            eval_expr("(get (list :one :two :three) 2)", &mut env),
            Ok(Form::keyword("three"))
        );
    }

    #[test]
    #[traced_test]
    #[ignore]
    fn eval_getn() {
        let mut env = std_env();

        assert!(matches!(
            eval_expr("(getn \"hello\" 0)", &mut env),
            Err(Error::UnexpectedArguments(_))
        ));

        assert_eq!(eval_expr("(getn (quote ()) 0)", &mut env), Ok(Form::Nil));

        assert_eq!(
            eval_expr("(getn (quote (1 2 3)) 1)", &mut env),
            Ok(Form::Int(2)),
        );

        assert_eq!(
            eval_expr(
                "(getn (list :one \"one\" :two \"two\" :three \"three\") :two)",
                &mut env
            ),
            Ok(Form::string("two"))
        );
    }

    #[test]
    #[traced_test]
    #[ignore]
    fn eval_map() {
        let mut env = std_env();

        eval_expr("(def echo (lambda (x) x))", &mut env).unwrap();
        eval_expr("(def zero (lambda (x) 0))", &mut env).unwrap();

        assert_eq!(
            eval_expr("(map (quote ()) (lambda (x) x))", &mut env),
            Ok(Form::List(vec![]))
        );

        assert_eq!(
            eval_expr("(map (quote (:one \"two\" 3)) echo)", &mut env),
            Ok(Form::List(vec![
                Form::keyword("one"),
                Form::string("two"),
                Form::Int(3),
            ]))
        );

        assert_eq!(
            eval_expr("(map (quote (1 2 3)) echo)", &mut env),
            Ok(Form::List(vec![Form::Int(1), Form::Int(2), Form::Int(3),]))
        );

        assert_eq!(
            eval_expr("(map (quote (1 2 3)) zero)", &mut env),
            Ok(Form::List(vec![Form::Int(0), Form::Int(0), Form::Int(0),]))
        );
    }

    #[test]
    #[traced_test]
    #[ignore]
    fn eval_push() {
        let mut env = std_env();

        eval_expr("(def my_lst (list))", &mut env).unwrap();

        assert_eq!(
            eval_expr("(push my_lst 1)", &mut env),
            Ok(Form::List(vec![Form::Int(1)])),
        );

        assert_eq!(
            eval_expr("my_lst", &mut env),
            Ok(Form::List(vec![Form::Int(1)])),
        );

        assert_eq!(
            eval_expr("(push my_lst :two)", &mut env),
            Ok(Form::List(vec![Form::Int(1), Form::keyword("two"),])),
        );

        assert_eq!(
            eval_expr("(push my_lst \"three\")", &mut env),
            Ok(Form::List(vec![
                Form::Int(1),
                Form::keyword("two"),
                Form::string("three"),
            ])),
        );

        assert_eq!(
            eval_expr("my_lst", &mut env),
            Ok(Form::List(vec![
                Form::Int(1),
                Form::keyword("two"),
                Form::string("three"),
            ])),
        )
    }

    #[test]
    #[traced_test]
    #[ignore]
    fn eval_pop() {
        let mut env = std_env();

        eval_expr("(def lst (list 1 2 3))", &mut env).unwrap();

        assert_eq!(eval_expr("(pop lst)", &mut env), Ok(Form::Int(3)));

        assert_eq!(
            eval_expr("lst", &mut env),
            Ok(Form::List(vec![Form::Int(1), Form::Int(2),]))
        );

        assert_eq!(eval_expr("(pop lst)", &mut env), Ok(Form::Int(2)));

        assert_eq!(eval_expr("(pop lst)", &mut env), Ok(Form::Int(1)));

        assert_eq!(eval_expr("(pop lst)", &mut env), Ok(Form::Nil));

        assert_eq!(eval_expr("lst", &mut env), Ok(Form::List(vec![])));
    }
}
