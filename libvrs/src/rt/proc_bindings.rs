//! Bindings for Process Fibers
use super::proc::{Extern, NativeFn, NativeFnVal, Val};
use super::proc_io::IOCmd;
use super::ProcessId;
use lyric::{Error, SymbolId};

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

/// Binding to get current process's PID
pub(crate) fn self_fn() -> NativeFn {
    NativeFn {
        symbol: SymbolId::from("self"),
        func: |f, _| {
            let pid = f.locals().pid;
            Ok(NativeFnVal::Return(Val::Int(pid as i32)))
        },
    }
}

/// Binding to list processes
pub(crate) fn ps_fn() -> NativeFn {
    NativeFn {
        symbol: SymbolId::from("ps"),
        func: |_, _| {
            Ok(NativeFnVal::Yield(Val::Extern(Extern::IOCmd(Box::new(
                IOCmd::ListProcesses,
            )))))
        },
    }
}

/// Binding to kill process
pub(crate) fn kill_fn() -> NativeFn {
    NativeFn {
        symbol: SymbolId::from("kill"),
        func: |_, args| {
            let pid = match args {
                [Val::Int(pid)] => pid,
                _ => {
                    return Err(Error::InvalidExpression(
                        "kill should have one integer argument".to_string(),
                    ))
                }
            };

            Ok(NativeFnVal::Yield(Val::Extern(Extern::IOCmd(Box::new(
                IOCmd::KillProcess(*pid as ProcessId),
            )))))
        },
    }
}

/// Binding to send messages
pub(crate) fn send_fn() -> NativeFn {
    NativeFn {
        symbol: SymbolId::from("send"),
        func: |_, args| {
            let (dst, msg) = match args {
                [Val::Int(dst), msg] => (dst, msg),
                _ => {
                    return Err(Error::InvalidExpression(
                        "Unexpected send call - (send DEST_PID DATA)".to_string(),
                    ))
                }
            };
            Ok(NativeFnVal::Yield(Val::Extern(Extern::IOCmd(Box::new(
                IOCmd::SendMessage(*dst as ProcessId, msg.clone()),
            )))))
        },
    }
}

/// Binding to list messages
pub(crate) fn ls_msgs_fn() -> NativeFn {
    NativeFn {
        symbol: SymbolId::from("ls-msgs"),
        func: |_, args| {
            if !args.is_empty() {
                return Err(Error::InvalidExpression(
                    "Unexpected ls-msgs call - No arguments expected".to_string(),
                ));
            }
            Ok(NativeFnVal::Yield(Val::Extern(Extern::IOCmd(Box::new(
                IOCmd::ListMessages,
            )))))
        },
    }
}
