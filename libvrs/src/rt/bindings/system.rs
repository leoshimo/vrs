//! Host System Bindings

use std::process::Stdio;

use crate::rt::program::{NativeAsyncFn, NativeFn, NativeFnOp, Val};
use lyric::{Error, Result};
use tokio::{io::AsyncReadExt, process::Command};
use tracing::{debug, error};

/// Binding for exec
pub(crate) fn exec_fn() -> NativeAsyncFn {
    NativeAsyncFn {
        doc: "(exec PROG ARG1 ARG2 ... ARGN) - Execute external executable PROG passing optional command line arguments ARG1 to ARGN.".to_string(),
        func: |_, args| Box::new(exec_impl(args)),
    }
}

/// Binding for shell_expand
pub(crate) fn shell_expand_fn() -> NativeFn {
    NativeFn {
        doc: "(shell_expand STRING) - Expand STRING using standard shell filename expansion."
            .to_string(),
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

/// Implementation of (exec PROG ARGS...)
async fn exec_impl(args: Vec<Val>) -> Result<Val> {
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

    debug!("exec {:?} {:?}", &prog, &args);

    let mut cmd = Command::new(prog.clone())
        .args(args.clone())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| Error::Runtime(format!("{e}")))?;

    let exit_status = cmd
        .wait()
        .await
        .map_err(|e| Error::Runtime(format!("{e}")))?;

    let mut output = cmd.stdout.take().expect("Expected stdout handle");
    let mut output_str = String::new();
    output
        .read_to_string(&mut output_str)
        .await
        .map_err(|e| Error::Runtime(format!("{e}")))?;
    let output_str = output_str.trim();

    if exit_status.success() {
        debug!("exec {:?} {:?} - {:?}", prog, args, exit_status);
        Ok(Val::List(vec![Val::keyword("ok"), Val::string(output_str)]))
    } else {
        error!("exec {:?} {:?} - {:?}", prog, args, exit_status);
        Err(Error::Runtime(format!(
            "Failed to execute - {}",
            exit_status
        )))
    }
}
