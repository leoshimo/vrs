// Test service runtime bindings

use vrs::{ProcessResult, Program, Runtime, Val};

#[tokio::test]
async fn srv_echo() {
    let rt = Runtime::new();

    let echo_prog = r#" (begin 
        (defn echo (name) (list "got" name))
        (srv :name :echo :interface '(echo))
    )"#;
    let echo_srv = Program::from_expr(echo_prog).unwrap();
    let _ = rt.run(echo_srv).await.unwrap();

    let req = Program::from_expr(
        r#"
        (list
            (call (find-srv :echo) '(:echo "one"))
            (call (find-srv :echo) '(:echo "two"))
            (call (find-srv :echo) '(:echo "three"))
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
    let rt = Runtime::new();

    let echo_prog = r#" (begin 
        (spawn (lambda () (begin
            (defn ping (msg) (list "pong" msg))
            (defn pong (msg) (list "ping" msg))
            (srv :name :ping_pong :interface '(ping pong)))))
        (list
            (call (find-srv :ping_pong) '(:ping "hi"))
            (call (find-srv :ping_pong) '(:pong "bye")))
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
    let rt = Runtime::new();

    let echo_prog = r#" (begin 
        (defn echo (name) (list "got" name))
        (srv :name :echo :interface '(echo))
    )"#;
    let echo_srv = Program::from_expr(echo_prog).unwrap();
    let _ = rt.run(echo_srv).await.unwrap();

    let req = Program::from_expr(r#"(call (find-srv :echo) '(:jibberish "one"))"#).unwrap();
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
    let rt = Runtime::new();

    let echo_prog = r#" (begin 
        (defn echo (name) (list "got" name))
        (srv :name :echo :interface '(echo))
    )"#;
    let echo_srv = Program::from_expr(echo_prog).unwrap();
    let _ = rt.run(echo_srv).await.unwrap();

    // no arg for :echo export
    let req = Program::from_expr(r#"(call (find-srv :echo) '(:echo))"#).unwrap();
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
    let rt = Runtime::new();

    // Spawn + interact on same program
    let prog = r#"(begin
         (spawn (lambda () (begin
            (defn echo (name) (list "got" name))
            (srv :name :echo :interface '(echo))
         )))
         (call (find-srv :echo) '(:echo "hello")))
    "#;
    let prog = Program::from_expr(prog).unwrap();
    let hdl = rt.run(prog).await.unwrap();

    let resp = hdl.join().await.unwrap();
    assert_eq!(
        resp.status.unwrap(),
        ProcessResult::Done(Val::List(vec![Val::string("got"), Val::string("hello"),]))
    );
}

// TODO: Test srv w/o :name errors
// TODO: Test srv w/o :interface errors
