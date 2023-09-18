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

/// Implements the `map` binding
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
        Value::List(v) => Ok(v),
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

// TODO: Write tests for list module
