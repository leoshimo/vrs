//! Bindings for Process Mailbox
use crate::rt::proc_io::IOCmd;
use crate::rt::program::{Extern, Lambda, NativeFn, NativeFnOp, Pattern, Val};
use lyric::{compile, parse, Error, SymbolId};

pub(crate) fn send_fn() -> NativeFn {
    NativeFn {
        func: |_, args| {
            let (dst, msg) = match args {
                [Val::Extern(Extern::ProcessId(dst)), msg] => (dst, msg),
                _ => {
                    return Err(Error::UnexpectedArguments(
                        "Unexpected send call - (send DEST_PID DATA)".to_string(),
                    ))
                }
            };
            Ok(NativeFnOp::Yield(Val::Extern(Extern::IOCmd(Box::new(
                IOCmd::SendMessage(*dst, msg.clone()),
            )))))
        },
    }
}

/// Binding to recv messages
pub(crate) fn recv_fn() -> NativeFn {
    NativeFn {
        func: |_, args| {
            let pattern = match args {
                [pat] => Some(Pattern::from_val(pat.clone())),
                [] => None,
                _ => {
                    return Err(Error::UnexpectedArguments(
                        "recv expects one or no arguments - (recv [PATTERN])".to_string(),
                    ))
                }
            };
            Ok(NativeFnOp::Yield(Val::Extern(Extern::IOCmd(Box::new(
                IOCmd::Recv(pattern),
            )))))
        },
    }
}

/// Binding to list messages
pub(crate) fn ls_msgs_fn() -> NativeFn {
    NativeFn {
        func: |_, args| {
            if !args.is_empty() {
                return Err(Error::UnexpectedArguments(
                    "Unexpected ls-msgs call - No arguments expected".to_string(),
                ));
            }
            Ok(NativeFnOp::Yield(Val::Extern(Extern::IOCmd(Box::new(
                IOCmd::ListMessages,
            )))))
        },
    }
}

/// Binding for call
pub(crate) fn call_fn() -> Lambda {
    Lambda {
        params: vec![SymbolId::from("pid"), SymbolId::from("msg")],
        code: compile(
            &parse(
                r#"
            (begin
                (def r (ref))
                (send pid (list r (self) msg))
                (get (recv (list r 'any)) 1))
        "#,
            )
            .unwrap()
            .into(),
        )
        .unwrap(),
        parent: None,
    }
}
