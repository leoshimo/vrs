//! E2E tests for Runtime and Client
use tracing_test::traced_test;
use vrs::{connection::Error, Client, Connection, Response, Runtime};

#[tokio::test]
#[traced_test]
async fn runtime_simple() {
    let runtime = Runtime::new();
    let (local, remote) = Connection::pair().unwrap();
    let mut remote = Client::new(remote);

    runtime
        .handle_conn(local)
        .await
        .expect("Connection should be handled");

    let resp = remote
        .request(lemma::Form::string("Hello world"))
        .await
        .expect("Request should succeed");

    assert_eq!(
        resp.contents,
        Ok(lemma::Form::string("Hello world")),
        "Sending request should succeed"
    );
}

#[tokio::test]
#[traced_test]
async fn runtime_remote_request_multi() {
    use lemma::parse as p;

    let runtime = Runtime::new();
    let (local, remote) = Connection::pair().unwrap();
    let mut client = Client::new(remote);

    runtime
        .handle_conn(local)
        .await
        .expect("Connection should be handled");

    assert!(
        matches!(
            client.request(p("(def message \"Hello world\")").unwrap()).await,
            Ok(Response { contents, .. }) if contents == Ok(lemma::Form::string("Hello world"))
        ),
        "defining a message binding should return its value"
    );
    assert!(
        matches!(
            client
                .request(p("(def echo (lambda (x) x))").unwrap())
                .await,
            Ok(Response { .. })
        ),
        "defining a echo binding is successful"
    );
    assert!(
        matches!(
            client.request(p("(echo message)").unwrap()).await,
            Ok(Response { contents, .. }) if contents == Ok(lemma::Form::string("Hello world"))
        ),
        "evaluating a function call passing defined argument symbols should return result"
    );

    let resp = client.request(p("jibberish").unwrap()).await.unwrap();
    assert!(
        matches!(
            resp.contents.expect_err("Should have errored"),
            Error::EvaluationError(_),
        ),
        "evaluating a jibberish underined symbol should return :err"
    );
}

// TODO: Test that each connection's process namespace is separate
