//! Conditional expressions
use crate::{Error, Extern, Locals, NativeFn, NativeFnOp, SymbolId, Val};

/// Language bindng for `eq?`
pub fn eq_fn<T: Extern, L: Locals>() -> NativeFn<T, L> {
    NativeFn {
        symbol: SymbolId::from("eq?"),
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

/// Language bindng for `contains`
pub fn contains_fn<T: Extern, L: Locals>() -> NativeFn<T, L> {
    NativeFn {
        symbol: SymbolId::from("contains"),
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
