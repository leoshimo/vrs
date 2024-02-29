//! Conditional expressions
use crate::{Error, Extern, Locals, NativeFn, NativeFnOp, Result, Val};

/// Language bindng for `eq?`
pub fn eq_fn<T: Extern, L: Locals>() -> NativeFn<T, L> {
    NativeFn {
        doc: "(eq? LHS RHS) - returns true if LHS equals RHS, otherwise false".to_string(),
        func: |_, args| {
            let (lhs, rhs) = match args {
                [lhs, rhs] => (lhs, rhs),
                _ => {
                    return Err(Error::UnexpectedArguments(
                        "eq? expects two arguments".to_string(),
                    ))
                }
            };
            Ok(NativeFnOp::Return(Val::Bool(lhs == rhs)))
        },
    }
}

/// Language binding for `contains?`
pub fn contains_fn<T: Extern, L: Locals>() -> NativeFn<T, L> {
    NativeFn {
        doc: "(contains? LIST ELEM) - returns true if LIST contains ELEM, otherwise false. LIST can be str or list".to_string(),
        func: |_, args| match args {
            [Val::List(l), target] => Ok(NativeFnOp::Return(Val::Bool(l.contains(target)))),
            [Val::String(s), Val::String(target)] => {
                Ok(NativeFnOp::Return(Val::Bool(s.contains(target))))
            }
            _ => Err(Error::UnexpectedArguments(
                "contains expects two arguments - (contains LST ELEM) or (contains STR SUBSTR)"
                    .to_string(),
            )),
        },
    }
}

/// Language binding for `not?`
pub fn not_fn<T: Extern, L: Locals>() -> NativeFn<T, L> {
    NativeFn {
        doc: "(not? EXPR) - returns the negation of truthiness of EXPR".to_string(),
        func: |_, args| match args {
            [cond] => Ok(NativeFnOp::Return(Val::Bool(!is_true(cond)?))),
            _ => Err(Error::UnexpectedArguments(
                "not expects single argument".to_string(),
            )),
        },
    }
}

/// Language binding for whether or not value is keyword
pub fn is_keyword_fn<T: Extern, L: Locals>() -> NativeFn<T, L> {
    NativeFn {
        doc: "(keyword? EXPR) - returns true if EXPR is keyword. Otherwise false".to_string(),
        func: |_, args| match args {
            [Val::Keyword(_)] => Ok(NativeFnOp::Return(Val::Bool(true))),
            [_] => Ok(NativeFnOp::Return(Val::Bool(false))),
            _ => Err(Error::UnexpectedArguments(
                "keyword? expects single argument".to_string(),
            )),
        },
    }
}

/// Defines true values
pub fn is_true<T: Extern, L: Locals>(v: &Val<T, L>) -> Result<bool> {
    let cond = match v {
        Val::Nil => false,
        Val::Bool(b) => *b,
        Val::Int(i) => *i != 0,
        Val::String(s) => !s.is_empty(),
        Val::List(l) => !l.is_empty(),
        v => {
            return Err(Error::UnexpectedArguments(format!(
                "Value is not a valid condition - {v}"
            )))
        }
    };
    Ok(cond)
}
