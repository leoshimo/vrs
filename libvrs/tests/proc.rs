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

#[tokio::test]
#[ignore] // TODO: Decide Isolation Policy / Fix
async fn spawn_env_lambda_isolated() {
    let rt = Runtime::new();

    let prog = r#"(begin
        (def parent_pid (self))

        (def a_var :parent)
        (defn set_var (val)
            (set a_var val))

        (spawn (fn ()
            (set_var :child)
            (send parent_pid :child_done)))

        (recv :child_done)
        a_var
    )"#;

    let hdl = rt.run(Program::from_expr(prog).unwrap()).await.unwrap();
    let exit = hdl.join().await.unwrap();

    assert_eq!(
        exit.status.unwrap(),
        ProcessResult::Done(Val::keyword("parent")),
        "calling set_var from spawned child should not affect parent's variables"
    );
}

/// Test nested lambdas for pseudo-objects
#[ignore] // TODO: Decide Isolation Policy / Fix
#[tokio::test]
async fn spawn_env_lambda_nested_isolated() {
    let rt = Runtime::new();

    let prog = r#"(begin
        (def parent_pid (self))

        (defn make_adder ()
            (def val 0)
            (lambda (x) (set val (+ val x))))

        (def adder (make_adder))
        (adder 2) # start both at 2

        (def child (spawn (fn ()
            (recv)
            (def result (adder 40))   # child should be 2 + 40 = 42
            (send parent_pid (list :child result)))))

        (adder 4) # parent edits isolated state, *post-spawn*
        (send child :start)

        (def (:child child_res) (recv))
        (def parent_res (adder 4))

        (list parent_res child_res))"#;

    let hdl = rt.run(Program::from_expr(prog).unwrap()).await.unwrap();
    let exit = hdl.join().await.unwrap();

    assert_eq!(
        exit.status.unwrap(),
        ProcessResult::Done(Val::List(vec![Val::Int(10), Val::Int(42),])),
        "calling set_var from spawned child should not affect parent's variables"
    );
}
