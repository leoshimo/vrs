//! Bindings for interacting with [Connection]
use crate::rt::proc_io::IOCmd;
use crate::rt::program::{Extern, Fiber, NativeFn, NativeFnOp, Val};
use lyric::{Error, Result};

/// Binding for `recv_req` to receive requests over client connection
pub(crate) fn recv_req_fn() -> NativeFn {
    NativeFn {
        func: |_, _| {
            Ok(NativeFnOp::Yield(Val::Extern(Extern::IOCmd(Box::new(
                IOCmd::RecvRequest,
            )))))
        },
    }
}

/// Binding for `send_resp` to send responses over client connection
pub(crate) fn send_resp_fn() -> NativeFn {
    NativeFn { func: send_resp }
}

/// Implements `send_resp`
fn send_resp(_f: &mut Fiber, args: &[Val]) -> Result<NativeFnOp> {
    let val = match args {
        [v] => v.clone(),
        _ => {
            return Err(Error::UnexpectedArguments(
                "send_conn expects two arguments".to_string(),
            ))
        }
    };
    Ok(NativeFnOp::Yield(Val::Extern(Extern::IOCmd(Box::new(
        IOCmd::SendResponse(val),
    )))))
}
