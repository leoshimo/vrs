//! Host System Bindings

use crate::rt::proc_io::IOCmd;
use crate::rt::program::{Extern, NativeFn, NativeFnOp, Val};
use lyric::{Error, Result};

/// Binding for exec
pub(crate) fn exec_fn() -> NativeFn {
    NativeFn {
        func: |_, args| {
            let (prog, args) = args.split_first().ok_or(Error::UnexpectedArguments(
                " Unexpected arguments to exec = (exec PROG [ARGS...])".to_string(),
            ))?;

            let prog = match prog {
                Val::String(s) => s.clone(),
                _ => {
                    return Err(Error::UnexpectedArguments(
                        "Expected string as first argument".to_string(),
                    ))
                }
            };

            let args = args
                .iter()
                .map(|a| match a {
                    Val::String(s) => Ok(s.clone()),
                    _ => Err(Error::UnexpectedArguments(
                        "exec can handle string arguments only".to_string(),
                    )),
                })
                .collect::<Result<Vec<_>>>()?;
            Ok(NativeFnOp::Yield(Val::Extern(Extern::IOCmd(Box::new(
                IOCmd::Exec(prog, args),
            )))))
        },
    }
}

/// Binding for shell_expand
pub(crate) fn shell_expand_fn() -> NativeFn {
    NativeFn {
        func: |_, args| {
            let path = match args {
                [Val::String(s)] => s,
                _ => {
                    return Err(Error::UnexpectedArguments(
                        "shell_expand expects one argument".to_string(),
                    ))
                }
            };
            let path = shellexpand::tilde(path).to_string();
            Ok(NativeFnOp::Return(Val::String(path)))
        },
    }
}
