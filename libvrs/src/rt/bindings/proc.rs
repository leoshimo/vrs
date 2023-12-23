//! Process Management Bindings
use crate::rt::proc_io::IOCmd;
use crate::rt::program::{Extern, Fiber, NativeAsyncFn, NativeFn, NativeFnOp, Program, Val};
use crate::rt::ProcessId;
use lyric::{Error, Result};
use std::time::Duration;
use tokio::time;
use tracing::debug;

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
pub(crate) fn ps_fn() -> NativeAsyncFn {
    NativeAsyncFn {
        func: |f, _| Box::new(ps_impl(f)),
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
pub(crate) fn sleep_fn() -> NativeAsyncFn {
    NativeAsyncFn {
        func: |_, args| {
            Box::new({
                async move {
                    let secs = match args[..] {
                        [Val::Int(secs)] => secs,
                        _ => {
                            return Err(Error::UnexpectedArguments(
                                "sleep expects single integer argument".to_string(),
                            ))
                        }
                    };
                    debug!("sleep secs = {:?}", secs);
                    time::sleep(Duration::from_secs(secs as u64)).await;
                    Ok(Val::keyword("ok"))
                }
            })
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

/// Implementation for ps
async fn ps_impl(fiber: &mut Fiber) -> Result<Val> {
    let kernel = fiber
        .locals()
        .kernel
        .as_ref()
        .and_then(|k| k.upgrade())
        .ok_or(Error::Runtime(
            "Kernel is missing for process".into(),
        ))?;
    let procs = kernel
        .procs()
        .await
        .map_err(|e| Error::Runtime(format!("{e}")))?
        .into_iter()
        .map(|pid| Val::Extern(Extern::ProcessId(pid)))
        .collect::<Vec<_>>();
    Ok(Val::List(procs))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rt::{kernel, ProcessResult};
    use assert_matches::assert_matches;

    #[tokio::test]
    async fn binding_self() {
        let k = kernel::start();
        let hdl = k
            .spawn_prog(Program::from_expr("(self)").unwrap())
            .await
            .expect("Kernel should spawn new process");

        let pid = hdl.id();
        let res = hdl.join().await.unwrap();
        assert_eq!(
            res.status.unwrap(),
            ProcessResult::Done(Val::Extern(Extern::ProcessId(pid)))
        );
    }

    #[tokio::test]
    async fn sleep() {
        let k = kernel::start();
        let hdl = k
            .spawn_prog(Program::from_expr("(sleep 0)").unwrap())
            .await
            .expect("Kernel should spawn new process");
        assert_eq!(k.procs().await.unwrap(), vec![hdl.id()]);

        let exit = hdl.join().await.unwrap();

        assert_eq!(
            exit.status.unwrap(),
            ProcessResult::Done(Val::keyword("ok"))
        );
    }

    #[tokio::test]
    async fn ps() {
        let k = kernel::start();
        let hdl = k
            .spawn_prog(Program::from_expr("(ps)").unwrap())
            .await
            .unwrap();

        let pid = Val::Extern(Extern::ProcessId(hdl.id()));
        let exit = hdl.join().await.unwrap();
        assert_matches!(
            exit.status.unwrap(),
            ProcessResult::Done(Val::List(pids)) if
                pids.contains(&pid)
        );
    }
}
