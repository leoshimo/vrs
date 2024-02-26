//! Builtins for types

use crate::{Error, Extern, Locals, NativeFn, NativeFnOp, Val};

pub(crate) fn ok_fn<T: Extern, L: Locals>() -> NativeFn<T, L> {
    NativeFn {
        doc: "(ok? FORM) - Returns false if FORM is an error value, otherwise true".to_string(),
        func: |_, args| {
            if args.len() != 1 {
                return Err(Error::UnexpectedArguments(
                    "ok? expects 1 argument".to_string(),
                ));
            }
            Ok(NativeFnOp::Return(Val::Bool(!matches!(
                args[0],
                Val::Error(_)
            ))))
        },
    }
}

pub(crate) fn err_fn<T: Extern, L: Locals>() -> NativeFn<T, L> {
    NativeFn {
        doc: "(err? FORM) - Returns true if FORM is an error value, otherwise false".to_string(),
        func: |_, args| {
            if args.len() != 1 {
                return Err(Error::UnexpectedArguments(
                    "err? expects 1 argument".to_string(),
                ));
            }
            Ok(NativeFnOp::Return(Val::Bool(matches!(
                args[0],
                Val::Error(_)
            ))))
        },
    }
}
