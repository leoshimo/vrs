//! Runtime tests    
use assert_matches::assert_matches;
use std::time::Duration;
use tokio::time::timeout;
use vrs::{Extern, ProcessResult, Program, Runtime, Val};

#[tokio::test]
async fn spawn_pid_is_different() {
    let rt = Runtime::new();

    let prog = r#"(begin
        (def origin_pid (self))
        (spawn (lambda () (send origin_pid (self))))
        (def spawn_pid (recv))
        (list origin_pid spawn_pid)
    )
    "#;
    let prog: Program = Program::from_expr(prog).unwrap();
    let hdl = rt.run(prog).await.unwrap();

    let exit = timeout(Duration::from_secs(0), hdl.join())
        .await
        .expect("shouldn't timeout")
        .unwrap();

    let pids = match exit.status.unwrap() {
        ProcessResult::Done(Val::List(pids)) => pids,
        _ => panic!("should be done w/ list of pids"),
    };

    assert_matches!(
        pids[..],
        [Val::Extern(Extern::ProcessId(origin)), Val::Extern(Extern::ProcessId(spawn))] if origin.inner() != spawn.inner()
    )
}

#[tokio::test]
async fn spawn_env_isolated() {
    let rt = Runtime::new();

    let prog = r#" (begin
        (def origin_pid (self))
        (def a_var :original)
        (spawn (lambda () (begin
            (set a_var :spawned)
            (send origin_pid a_var))))
        (def spawn_var (recv))
        (list a_var spawn_var) # a_var should not be overridden
    )
    "#;

    let prog = Program::from_expr(prog).unwrap();
    let hdl = rt.run(prog).await.unwrap();

    let exit = timeout(Duration::from_secs(0), hdl.join())
        .await
        .expect("Should not timeout")
        .unwrap();

    let values = match exit.status.unwrap() {
        ProcessResult::Done(Val::List(values)) => values,
        _ => panic!("Should be done with list of values"),
    };

    assert_eq!(
        values,
        vec![Val::keyword("original"), Val::keyword("spawned")],
        "Spawning new variable should have isolated state"
    );
}

// TODO: Write  Test case for Service Registry + Discovery
// Cover - =register=, =ls-srv=, =find-srv=, deregistration on complete
//
// - Unknown service return nil
//
// Proc A:
// Running `target/debug/vrsctl`
// vrs> (ls-srv)
// ()
// vrs> (register :hello)
// :ok
// vrs> (ls-srv)
// ((:name :hello :pid <pid 1>))
//
// Proc B:
// vrs> (ls-srv)
// ((:name :hello :pid <pid 1>))
// vrs> (get (ls-srv) 0)
// (:name :hello :pid <pid 1>)
// vrs> (get (get (ls-srv) 0) :pid)
// <pid 1>
// vrs> (send (get (get (ls-srv) 0) :pid) :hi)
// :hi
// vrs>
//
