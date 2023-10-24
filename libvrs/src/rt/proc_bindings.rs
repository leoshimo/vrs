//! Bindings for Process Fibers
use super::proc::{Extern, NativeFn, NativeFnVal, Val};
use super::proc_io::IOCmd;
use lyric::SymbolId;

/// Binding for `recv_req` to receive requests over client connection
pub(crate) fn recv_req_fn() -> NativeFn {
    NativeFn {
        symbol: SymbolId::from("recv_req"),
        func: |_, _| {
            Ok(NativeFnVal::Yield(Val::Extern(Extern::IOCmd(Box::new(
                IOCmd::RecvRequest,
            )))))
        },
    }
}

/// Binding for `send_resp` to send responses over client connection
pub(crate) fn send_resp_fn() -> NativeFn {
    NativeFn {
        symbol: SymbolId::from("send_resp"),
        func: |_, args| -> std::result::Result<NativeFnVal, lyric::Error> {
            let val = match args {
                [v] => v.clone(),
                _ => {
                    return Err(lyric::Error::InvalidExpression(
                        "send_conn expects two arguments".to_string(),
                    ))
                }
            };
            Ok(NativeFnVal::Yield(Val::Extern(Extern::IOCmd(Box::new(
                IOCmd::SendRequest(val),
            )))))
        },
    }
}

/// Bindings to get current process's PID
pub(crate) fn self_fn() -> NativeFn {
    NativeFn {
        symbol: SymbolId::from("self"),
        func: |f, _| {
            let pid = f.locals().pid;
            Ok(NativeFnVal::Return(Val::Int(pid as i32)))
        },
    }
}
