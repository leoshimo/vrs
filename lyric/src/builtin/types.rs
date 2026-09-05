//! Builtins for types

use crate::{Error, Extern, Locals, NativeFn, NativeFnOp, Val};

pub(crate) fn list_predicate_fn<T: Extern, L: Locals>() -> NativeFn<T, L> {
    NativeFn {
        metadata: vec![],
        doc: "(list? VALUE) - Whether VALUE is a list".into(),
        func: |_, args| match args {
            [value] => Ok(NativeFnOp::Return(Val::Bool(matches!(value, Val::List(_))))),
            _ => Err(Error::UnexpectedArguments(
                "list? expects one argument".into(),
            )),
        },
    }
}

pub(crate) fn error_fn<T: Extern, L: Locals>() -> NativeFn<T, L> {
    NativeFn {
        metadata: vec![],
        doc: "(error MESSAGE) - Raise an error, catchable with try".into(),
        func: |_, args| match args {
            [Val::String(message)] => Err(Error::Runtime(message.clone())),
            _ => Err(Error::UnexpectedArguments(
                "error expects a message string".into(),
            )),
        },
    }
}

pub(crate) fn ok_fn<T: Extern, L: Locals>() -> NativeFn<T, L> {
    NativeFn {
        metadata: vec![],
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
        metadata: vec![],
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
