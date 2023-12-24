//! Process Management Bindings
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
pub(crate) fn kill_fn() -> NativeAsyncFn {
    NativeAsyncFn {
        func: |f, args| Box::new(kill_impl(f, args)),
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
pub(crate) fn spawn_fn() -> NativeAsyncFn {
    NativeAsyncFn {
        func: |f, args| Box::new(spawn_impl(f, args)),
    }
}

/// Implementation for (ps)
async fn ps_impl(fiber: &mut Fiber) -> Result<Val> {
    let kernel = fiber
        .locals()
        .kernel
        .as_ref()
        .and_then(|k| k.upgrade())
        .ok_or(Error::Runtime("Kernel is missing for process".into()))?;
    let procs = kernel
        .procs()
        .await
        .map_err(|e| Error::Runtime(format!("{e}")))?
        .into_iter()
        .map(|pid| Val::Extern(Extern::ProcessId(pid)))
        .collect::<Vec<_>>();
    Ok(Val::List(procs))
}

/// Implementation for (kill PID)
async fn kill_impl(fiber: &mut Fiber, args: Vec<Val>) -> Result<Val> {
    let pid = match args[..] {
        [Val::Extern(Extern::ProcessId(pid))] => pid,
        [Val::Int(pid)] => ProcessId::from(pid as usize),
        _ => {
            return Err(Error::UnexpectedArguments(
                "kill should have one integer argument".to_string(),
            ))
        }
    };
    let kernel = fiber
        .locals()
        .kernel
        .as_ref()
        .and_then(|k| k.upgrade())
        .ok_or(Error::Runtime("Kernel is missing for process".to_string()))?;
    kernel
        .kill_proc(pid)
        .await
        .map_err(|e| Error::Runtime(format!("{e}")))?;
    Ok(Val::keyword("ok"))
}

/// Implementation for (spawn PROG)
async fn spawn_impl(fiber: &mut Fiber, args: Vec<Val>) -> Result<Val> {
    let lambda = match args.as_slice() {
        [Val::Lambda(l)] => l.clone(),
        _ => {
            return Err(Error::UnexpectedArguments(
                "spawn expects single lambda".to_string(),
            ))
        }
    };
    let prog = Program::from_lambda(lambda)?;
    let kernel = fiber
        .locals()
        .kernel
        .as_ref()
        .and_then(|k| k.upgrade())
        .ok_or(Error::Runtime("Kernel is missing for process".to_string()))?;
    let hdl = kernel
        .spawn_prog(prog)
        .await
        .map_err(|e| Error::Runtime(format!("{e}")))?;
    Ok(Val::Extern(Extern::ProcessId(hdl.id())))
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

    #[tokio::test]
    async fn kill() {
        use tokio::time;

        let k = kernel::start();

        let kill_target = k
            .spawn_prog(Program::from_expr("(loop (sleep 0))").unwrap())
            .await
            .unwrap();

        let kill_src = k
            .spawn_prog(
                Program::from_expr(&format!("(kill (pid {}))", kill_target.id().inner())).unwrap(),
            )
            .await
            .unwrap();

        kill_src.join().await.expect("kill_src should terminate");

        let killed_exit = time::timeout(Duration::from_millis(0), kill_target.join())
            .await
            .expect("kill_target process should terminate")
            .unwrap();
        assert_eq!(killed_exit.status.unwrap(), ProcessResult::Cancelled);
    }

    #[tokio::test]
    async fn binding_spawn() {
        let k = kernel::start();

        let prog = r#"(begin
            (spawn (lambda () (loop (sleep 0)))) # spawn infinite loop
        )"#;
        let hdl = k
            .spawn_prog(Program::from_expr(prog).unwrap())
            .await
            .unwrap();

        let origin_pid = hdl.id();

        let exit = hdl.join().await.unwrap();

        let spawned_pid = match exit.status.unwrap() {
            ProcessResult::Done(Val::Extern(Extern::ProcessId(spawned_pid))) => spawned_pid,
            _ => panic!("Process should terminate w/ PID return value of (spawn ...)"),
        };

        assert!(
            origin_pid != spawned_pid,
            "Origin and spawned PID should be different"
        );

        let running = k.procs().await.unwrap();
        assert!(
            !running.contains(&origin_pid),
            "Origin pid should be terminated"
        );
        assert!(
            running.contains(&spawned_pid),
            "Spawned pid should still be running"
        );
    }
}
