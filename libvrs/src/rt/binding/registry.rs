//! Bindings for interacting with [Registry]

use crate::rt::proc_io::IOCmd;
use crate::rt::program::{Extern, NativeFn, NativeFnOp, Val};
use lyric::Error;

/// Binding for register
pub(crate) fn register_fn() -> NativeFn {
    NativeFn {
        func: |_, args| {
            let keyword = match args {
                [Val::Keyword(k)] => k.clone(),
                _ => {
                    return Err(Error::UnexpectedArguments(
                        "register expects single keyword argument".to_string(),
                    ))
                }
            };

            Ok(NativeFnOp::Yield(Val::Extern(Extern::IOCmd(Box::new(
                IOCmd::RegisterAsService(keyword),
            )))))
        },
    }
}

/// Binding for ls-srv
pub(crate) fn ls_srv_fn() -> NativeFn {
    NativeFn {
        func: |_, args| {
            if !args.is_empty() {
                return Err(Error::UnexpectedArguments(
                    "ls-srv expects single keyword argument".to_string(),
                ));
            }
            Ok(NativeFnOp::Yield(Val::Extern(Extern::IOCmd(Box::new(
                IOCmd::ListServices,
            )))))
        },
    }
}

/// Binding for find-srv
pub(crate) fn find_srv_fn() -> NativeFn {
    NativeFn {
        func: |_, args| {
            let keyword = match args {
                [Val::Keyword(k)] => k.clone(),
                _ => {
                    return Err(Error::UnexpectedArguments(
                        "find-srv expects single keyword argument".to_string(),
                    ))
                }
            };

            Ok(NativeFnOp::Yield(Val::Extern(Extern::IOCmd(Box::new(
                IOCmd::FindService(keyword),
            )))))
        },
    }
}
