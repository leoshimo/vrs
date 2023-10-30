//! Bindings for Process Fibers

use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::proc::{Env, Extern, Lambda, NativeFn, NativeFnOp, Val};
use super::proc_io::IOCmd;
use super::ProcessId;
use lyric::{compile, parse, Error, Pattern, Result, SymbolId};

/// Binding for `recv_req` to receive requests over client connection
pub(crate) fn recv_req_fn() -> NativeFn {
    NativeFn {
        symbol: SymbolId::from("recv_req"),
        func: |_, _| {
            Ok(NativeFnOp::Yield(Val::Extern(Extern::IOCmd(Box::new(
                IOCmd::RecvRequest,
            )))))
        },
    }
}

/// Binding for `send_resp` to send responses over client connection
pub(crate) fn send_resp_fn() -> NativeFn {
    NativeFn {
        symbol: SymbolId::from("send_resp"),
        func: |_, args| {
            let val = match args {
                [v] => v.clone(),
                _ => {
                    return Err(Error::UnexpectedArguments(
                        "send_conn expects two arguments".to_string(),
                    ))
                }
            };
            Ok(NativeFnOp::Yield(Val::Extern(Extern::IOCmd(Box::new(
                IOCmd::SendRequest(val),
            )))))
        },
    }
}

/// binding to create a new PID
pub(crate) fn pid_fn() -> NativeFn {
    NativeFn {
        symbol: SymbolId::from("pid"),
        func: |_, args| {
            let pid = match args {
                [Val::Int(pid)] => pid,
                _ => {
                    return Err(Error::UnexpectedArguments(
                        "pid expects single integer argument".to_string(),
                    ))
                }
            };
            Ok(NativeFnOp::Return(Val::Extern(Extern::ProcessId(
                ProcessId::from(*pid as usize),
            ))))
        },
    }
}

/// binding to get current process's pid
pub(crate) fn self_fn() -> NativeFn {
    NativeFn {
        symbol: SymbolId::from("self"),
        func: |f, _| {
            let pid = f.locals().pid;
            Ok(NativeFnOp::Return(Val::Extern(Extern::ProcessId(pid))))
        },
    }
}

/// Binding to list processes
pub(crate) fn ps_fn() -> NativeFn {
    NativeFn {
        symbol: SymbolId::from("ps"),
        func: |_, _| {
            Ok(NativeFnOp::Yield(Val::Extern(Extern::IOCmd(Box::new(
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
                [Val::Extern(Extern::ProcessId(pid))] => *pid,
                [Val::Int(pid)] => ProcessId::from(*pid as usize),
                _ => {
                    return Err(Error::UnexpectedArguments(
                        "kill should have one integer argument".to_string(),
                    ))
                }
            };

            Ok(NativeFnOp::Yield(Val::Extern(Extern::IOCmd(Box::new(
                IOCmd::KillProcess(pid),
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
        symbol: SymbolId::from("recv"),
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
        symbol: SymbolId::from("ls-msgs"),
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

/// Binding for exec
pub(crate) fn exec_fn() -> NativeFn {
    NativeFn {
        symbol: SymbolId::from("exec"),
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

// TODO: call_fn, open_url_fn, open_app_fn, open - need easier binding options (init script?)
/// Binding for call
pub(crate) fn call_fn(env: Arc<Mutex<Env>>) -> Lambda {
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
        env,
    }
}

/// Binding for open_url
pub(crate) fn open_url_fn(env: Arc<Mutex<Env>>) -> Lambda {
    Lambda {
        params: vec![SymbolId::from("url")],
        code: compile(&parse(r#"(exec "open" "-a" "Safari" url)"#).unwrap().into()).unwrap(),
        env,
    }
}

/// Binding for open_app
pub(crate) fn open_app_fn(env: Arc<Mutex<Env>>) -> Lambda {
    Lambda {
        params: vec![SymbolId::from("app")],
        code: compile(&parse(r#"(exec "open" "-a" app)"#).unwrap().into()).unwrap(),
        env,
    }
}

/// Binding for shell_expand
pub(crate) fn shell_expand_fn() -> NativeFn {
    NativeFn {
        symbol: SymbolId::from("shell_expand"),
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

/// Binding for open_file
pub(crate) fn open_file_fn(env: Arc<Mutex<Env>>) -> Lambda {
    Lambda {
        params: vec![SymbolId::from("file")],
        code: compile(
            &parse(r#"(exec "open" (shell_expand file))"#)
                .unwrap()
                .into(),
        )
        .unwrap(),
        env,
    }
}

/// Binding for sleep
pub(crate) fn sleep_fn() -> NativeFn {
    NativeFn {
        symbol: SymbolId::from("sleep"),
        func: |_, args| {
            let secs = match args {
                [Val::Int(secs)] => secs,
                _ => {
                    return Err(Error::UnexpectedArguments(
                        "sleep expects single integer argument".to_string(),
                    ))
                }
            };
            Ok(NativeFnOp::Yield(Val::Extern(Extern::IOCmd(Box::new(
                IOCmd::Sleep(Duration::from_secs(*secs as u64)),
            )))))
        },
    }
}

/// Binding for spawn
pub(crate) fn spawn_fn() -> NativeFn {
    NativeFn {
        symbol: SymbolId::from("spawn"),
        func: |_, args| {
            let prog = match args {
                [prog] => prog.clone(),
                _ => {
                    return Err(Error::UnexpectedArguments(
                        "spawn expects single expression".to_string(),
                    ))
                }
            };
            Ok(NativeFnOp::Yield(Val::Extern(Extern::IOCmd(Box::new(
                IOCmd::Spawn(prog),
            )))))
        },
    }
}
