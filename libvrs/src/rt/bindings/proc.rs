//! Process Management Bindings
use crate::rt::proc_io::IOCmd;
use crate::rt::program::{Extern, NativeFn, NativeFnOp, Program, Val};
use crate::rt::ProcessId;
use lyric::Error;
use std::time::Duration;

/// binding to get current process's pid
pub(crate) fn self_fn() -> NativeFn {
    NativeFn {
        func: |f, _| {
            let pid = f.locals().pid;
            Ok(NativeFnOp::Return(Val::Extern(Extern::ProcessId(pid))))
        },
    }
}

/// binding to create a new PID
pub(crate) fn pid_fn() -> NativeFn {
    NativeFn {
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

/// Binding to list processes
pub(crate) fn ps_fn() -> NativeFn {
    NativeFn {
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

/// Binding for sleep
pub(crate) fn sleep_fn() -> NativeFn {
    NativeFn {
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
        func: |_, args| {
            let lambda = match args {
                [Val::Lambda(l)] => l.clone(),
                _ => {
                    return Err(Error::UnexpectedArguments(
                        "spawn expects single lambda".to_string(),
                    ))
                }
            };

            let prog = Program::from_lambda(lambda)?;
            Ok(NativeFnOp::Yield(Val::Extern(Extern::IOCmd(Box::new(
                IOCmd::Spawn(prog),
            )))))
        },
    }
}
