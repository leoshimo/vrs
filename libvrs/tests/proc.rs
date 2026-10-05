//! Runtime tests    
use assert_matches::assert_matches;
use std::time::Duration;
use tokio::time::timeout;
use vrs::{Extern, ProcessResult, Program, Runtime, Val};

#[tokio::test]
async fn spawn_pid_is_different() {
    let rt = Runtime::new("test");

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
        &pids[..],
        [Val::Extern(Extern::ProcessId(origin)), Val::Extern(Extern::ProcessId(spawn))] if origin.inner() != spawn.inner()
    )
}

#[tokio::test]
async fn spawn_env_isolated() {
    let rt = Runtime::new("test");

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
async fn spawn_env_lambda_isolated() {
    let rt = Runtime::new("test");

    let prog = r#"(begin
        (def parent_pid (self))

        (def a_var :parent)
        (defn! set_var (val)
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
#[tokio::test]
async fn spawn_env_lambda_nested_isolated() {
    let rt = Runtime::new("test");

    let prog = r#"(begin
        (def parent_pid (self))

        (defn! make_adder ()
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

async fn run_isolation(source: &str) -> Val {
    let rt = Runtime::new("test");
    let handle = rt.run(Program::from_expr(source).unwrap()).await.unwrap();
    timeout(Duration::from_secs(5), handle.join())
        .await
        .expect("isolation program should finish")
        .unwrap()
        .status
        .unwrap()
        .unwrap()
}

#[tokio::test]
async fn spawn_preserves_shared_captures_recursion_and_root_identity() {
    let result = run_isolation(
        r#"(begin
        (def parent (self))
        (def count 0)
        (defn! step () (set count (+ count 1)))
        (defn! recur (n) (if (eq? n 0) count (begin (step) (recur (- n 1)))))
        (def (inc read) ((fn ()
            (def count 10)
            (list (fn () (set count (+ count 1))) (fn () count)))))
        (spawn (fn ()
            (set count 20)
            (inc)
            (send parent (list (recur 2) (read) count))))
        (list (recv) count (read)))"#,
    )
    .await;
    assert_eq!(result, Val::from_expr("((22 11 22) 0 10)").unwrap());
}

#[tokio::test]
async fn messages_reject_functions_including_nested_values_and_self_sends() {
    let result = run_isolation(
        r#"(begin
        (def count 0)
        (def callback (fn () (set count (+ count 1))))
        (def child (spawn (fn () (recv))))
        (def errors (list
            (err? (try (send child callback)))
            (err? (try (send child (list :nested (list callback)))))
            (err? (try (send (self) callback)))
            (err? (try (send (self) +)))
            (err? (try (send (self) send)))
            (err? (try (call child (list :execute callback))))))
        (send child :finish)
        (list errors count (ls_msgs)))"#,
    )
    .await;
    assert_eq!(
        result,
        Val::from_expr("((true true true true true true) 0 ())").unwrap()
    );
}

#[tokio::test]
async fn service_returns_error_for_function_results_and_keeps_serving() {
    let result = run_isolation(
        r#"(begin
        (defn! callback () (list :nested (fn () 42)))
        (defn! ping () :pong)
        (def child (spawn_srv! :data_only :interface '(callback ping)))
        (list (err? (call child '(:callback))) (call child '(:ping))))"#,
    )
    .await;
    assert_eq!(result, Val::from_expr("(true :pong)").unwrap());
}

#[tokio::test]
async fn publications_reject_functions_without_delivering_them() {
    let result = run_isolation(
        r#"(begin
        (subscribe :data_only)
        (def rejected (err? (try (publish :data_only (list (fn () 42))))))
        (publish :data_only '(:count 1))
        (list rejected (recv) (ls_msgs)))"#,
    )
    .await;
    assert_eq!(
        result,
        Val::from_expr("(true (:topic_updated :data_only (:count 1)) ())").unwrap()
    );
}

#[tokio::test]
async fn child_retains_its_captures_after_parent_exits() {
    let rt = Runtime::new("test");
    let parent = rt
        .run(
            Program::from_expr(
                r#"(begin
        (def count 41)
        (defn! read () (+ count 1))
        (spawn (fn ()
            (def (request caller _) (recv))
            (send caller (list request (read))))))"#,
            )
            .unwrap(),
        )
        .await
        .unwrap();
    let child = parent.join().await.unwrap().status.unwrap().unwrap();
    let caller = rt
        .run(
            Program::from_val(Val::List(vec![
                Val::symbol("call"),
                child,
                Val::keyword("read"),
            ]))
            .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        timeout(Duration::from_secs(5), caller.join())
            .await
            .unwrap()
            .unwrap()
            .status
            .unwrap(),
        ProcessResult::Done(Val::Int(42))
    );
}

#[tokio::test]
async fn spawn_copies_macro_captures_together_with_ordinary_functions() {
    let result = run_isolation(
        r#"(begin
        (def parent (self))
        (def count 0)
        (defn! read () count)
        (defmacro bump () (set count (+ count 1)) count)
        (def read_hidden ((fn ()
            (def hidden 10)
            (defmacro bump_hidden () (set hidden (+ hidden 1)) hidden)
            (fn () hidden))))
        (spawn (fn ()
            (set count 20)
            (send parent (list (bump!) (read) count (bump_hidden!) (read_hidden)))))
        (list (recv) count (read_hidden)))"#,
    )
    .await;
    assert_eq!(result, Val::from_expr("((21 21 21 11 11) 0 10)").unwrap());
}

#[tokio::test]
async fn spawned_services_have_independent_state() {
    let result = run_isolation(
        r#"(begin
        (def count 0)
        (defn! inc () (set count (+ count 1)))
        (defn! read () count)
        (def first_service (spawn_srv! :first_counter :interface '(inc read)))
        (def second_service (spawn_srv! :second_counter :interface '(inc read)))
        (call first_service '(:inc))
        (list (call first_service '(:read)) (call second_service '(:read)) count))"#,
    )
    .await;
    assert_eq!(result, Val::from_expr("(1 0 0)").unwrap());
}
