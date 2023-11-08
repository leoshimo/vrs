// Test service runtime bindings

use vrs::{ProcessResult, Program, Runtime, Val};

#[tokio::test]
async fn srv_echo() {
    let rt = Runtime::new();

    let echo_prog = r#" (begin 
        (defn echo (name) (list "got" name))
        (srv :name :echo :exports '(echo))
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
async fn srv_echo_invalid_msg() {
    let rt = Runtime::new();

    let echo_prog = r#" (begin 
        (defn echo (name) (list "got" name))
        (srv :name :echo :exports '(echo))
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
        (srv :name :echo :exports '(echo))
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

// TODO: Test calling w/ invalid number of arguments
