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
