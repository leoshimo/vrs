// Test service macros and runtime bindings

use vrs::{ProcessResult, Program, Runtime, Val};

async fn run_service_program(source: &str) -> Val {
    let rt = Runtime::new("test");
    let handle = rt.run(Program::from_expr(source).unwrap()).await.unwrap();
    let result = tokio::time::timeout(std::time::Duration::from_secs(5), handle.join())
        .await
        .expect("service program should finish")
        .unwrap();
    let ProcessResult::Done(value) = result.status.unwrap() else {
        panic!("service program should return a value");
    };
    value
}

#[tokio::test]
async fn service_topics_are_ready_and_share_state_with_calls() {
    let result = run_service_program(
        r#"(begin
      (def parent (self))
      (def evaluations 0)
      (def received '())
      (defn! remember (data)
        (set received (push received data))
        (send parent (list :handled (self) data)))
      (defn! snapshot () received)
      (def service (spawn_srv! :events
        :topics (begin (set evaluations (+ evaluations 1)) '((:changed remember)))
        :interface '(snapshot)))
      (publish :changed '(:completed :id 7 :command (+ 1 2)))
      (def ack (recv (list :handled service '_)))
      (list evaluations (get ack 2) (call service '(:snapshot))))"#,
    )
    .await;
    assert_eq!(
        result,
        Val::from_expr(
            "(1 (:completed :id 7 :command (+ 1 2)) ((:completed :id 7 :command (+ 1 2))))"
        )
        .unwrap()
    );
}

#[tokio::test]
async fn service_dispatch_survives_event_errors_and_uses_current_handler() {
    let result = run_service_program(
        r#"(begin
      (def parent (self))
      (def service (spawn (fn ()
      (defn! event (data)
        (if (eq? data :fail) (error "event failed"))
        (send parent (list :handled data)))
      (defn! replace ()
        (set event (fn (data) (send parent (list :replaced data)))))
      (defn! ping () :pong)
      (srv! :events :interface '(replace ping)
        :topics '((:first event) (:second event)) :ready parent))))
      (recv (list :service_ready service))
      (send service '(:topic_updated :first :fail))
      (send service :unrelated)
      (send service '(:topic_updated :unknown :ignored))
      (send service '(:unrelated :triple :ignored))
      (send service '(:not_a_request :not_a_pid (:replace)))
      (publish :second :ok)
      (def first (recv '(:handled _)))
      (call service '(:replace))
      (send service '(:topic_updated :first :new))
      (list first (recv '(:replaced _)) (call service '(:ping))))"#,
    )
    .await;
    assert_eq!(
        result,
        Val::from_expr("((:handled :ok) (:replaced :new) :pong)").unwrap()
    );
}

#[tokio::test]
async fn current_process_service_supports_topics_without_exported_calls() {
    let result = run_service_program(
        r#"(begin
      (def parent (self))
      (def child (spawn (fn ()
        (defn! report (report) (send parent (list :payload report)))
        (srv! :listener :ready parent :topics '((:value report)) :interface '()))))
      (recv (list :service_ready child))
      (publish :value '(one two))
      (recv '(:payload _)))"#,
    )
    .await;
    assert_eq!(result, Val::from_expr("(:payload (one two))").unwrap());
}

#[tokio::test]
async fn srv_echo() {
    let rt = Runtime::new("test");

    let echo_prog = r#" (begin 
        (defn! echo (name) (list "got" name))
        (srv! :echo :interface '(echo))
    )"#;
    let echo_srv = Program::from_expr(echo_prog).unwrap();
    let _ = rt.run(echo_srv).await.unwrap();

    let req = Program::from_expr(
        r#"
        (list
            (call (find_srv :echo) '(:echo "one"))
            (call (find_srv :echo) '(:echo "two"))
            (call (find_srv :echo) '(:echo "three"))
        )"#,
    )
    .unwrap();
    let req = rt.run(req).await.unwrap();

    let resp = req.join().await.unwrap();
    assert_eq!(
        resp.status.unwrap(),
        ProcessResult::Done(Val::List(vec![
            Val::List(vec![Val::string("got"), Val::string("one")]),
            Val::List(vec![Val::string("got"), Val::string("two")]),
            Val::List(vec![Val::string("got"), Val::string("three")]),
        ]))
    );
}

#[tokio::test]
async fn srv_multi_interface() {
    let rt = Runtime::new("test");

    let echo_prog = r#" (begin 
        (spawn (lambda () (begin
            (defn! ping (msg) (list "pong" msg))
            (defn! pong (msg) (list "ping" msg))
            (srv! :ping_pong :interface '(ping pong)))))
        (list
            (call (find_srv :ping_pong) '(:ping "hi"))
            (call (find_srv :ping_pong) '(:pong "bye")))
    )"#;
    let prog = Program::from_expr(echo_prog).unwrap();
    let hdl = rt.run(prog).await.unwrap();

    let resp = hdl.join().await.unwrap();
    assert_eq!(
        resp.status.unwrap(),
        ProcessResult::Done(Val::List(vec![
            Val::List(vec![Val::string("pong"), Val::string("hi")]),
            Val::List(vec![Val::string("ping"), Val::string("bye")]),
        ]))
    );
}

#[tokio::test]
async fn srv_echo_invalid_msg() {
    let rt = Runtime::new("test");

    let echo_prog = r#" (begin 
        (defn! echo (name) (list "got" name))
        (srv! :echo :interface '(echo))
    )"#;
    let echo_srv = Program::from_expr(echo_prog).unwrap();
    let _ = rt.run(echo_srv).await.unwrap();

    let req = Program::from_expr(r#"(call (find_srv :echo) '(:jibberish "one"))"#).unwrap();
    let req = rt.run(req).await.unwrap();

    let resp = req.join().await.unwrap();
    assert_eq!(
        resp.status.unwrap(),
        ProcessResult::Done(Val::List(vec![
            Val::keyword("err"),
            Val::string("Unrecognized message")
        ]))
    );
}

#[tokio::test]
async fn srv_echo_invalid_arg() {
    let rt = Runtime::new("test");

    let echo_prog = r#" (begin 
        (defn! echo (name) (list "got" name))
        (srv! :echo :interface '(echo))
    )"#;
    let echo_srv = Program::from_expr(echo_prog).unwrap();
    let _ = rt.run(echo_srv).await.unwrap();

    // no arg for :echo export
    let req = Program::from_expr(r#"(call (find_srv :echo) '(:echo))"#).unwrap();
    let req = rt.run(req).await.unwrap();

    let resp = req.join().await.unwrap();
    assert_eq!(
        resp.status.unwrap(),
        ProcessResult::Done(Val::List(vec![
            Val::keyword("err"),
            Val::string("Unrecognized message")
        ]))
    );
}

#[tokio::test]
async fn spawn_echo_svc() {
    let rt = Runtime::new("test");

    // Spawn + interact on same program
    let prog = r#"(begin
         (spawn (lambda () (begin
            (defn! echo (name) (list "got" name))
            (srv! :echo :interface '(echo))
         )))
         (call (find_srv :echo) '(:echo "hello")))
    "#;
    let prog = Program::from_expr(prog).unwrap();
    let hdl = rt.run(prog).await.unwrap();

    let resp = hdl.join().await.unwrap();
    assert_eq!(
        resp.status.unwrap(),
        ProcessResult::Done(Val::List(vec![Val::string("got"), Val::string("hello"),]))
    );
}

#[tokio::test]
async fn spawn_srv_returns_after_service_registration() {
    let rt = Runtime::new("test");
    let prog = Program::from_expr(
        r#"(begin
            (defn! ping () :pong)
            (def spawned (spawn_srv! :ready_probe :interface '(ping)))
            (list spawned (find_srv :ready_probe) (call spawned '(:ping))))"#,
    )
    .unwrap();
    let result = rt.run(prog).await.unwrap().join().await.unwrap();

    let ProcessResult::Done(Val::List(values)) = result.status.unwrap() else {
        panic!("expected spawn result, registry result, and service response");
    };
    assert_eq!(values[0], values[1]);
    assert_eq!(values[2], Val::keyword("pong"));
}
