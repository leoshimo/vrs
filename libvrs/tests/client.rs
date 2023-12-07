//! E2E tests for Runtime and Client
use vrs::{Client, Connection, Response, Runtime};

#[tokio::test]
async fn runtime_simple() {
    let runtime = Runtime::new();
    let (local, remote) = Connection::pair().unwrap();
    let remote = Client::new(remote);

    runtime
        .handle_conn(local)
        .await
        .expect("Connection should be handled");

    let resp = remote
        .request(lyric::Form::string("Hello world"))
        .await
        .expect("Request should succeed");

    assert_eq!(
        resp.contents,
        Ok(lyric::Form::string("Hello world")),
        "Sending request should succeed"
    );
}

#[tokio::test]
async fn runtime_remote_request_multi() {
    use lyric::parse as p;

    let runtime = Runtime::new();
    let (local, remote) = Connection::pair().unwrap();
    let client = Client::new(remote);

    runtime
        .handle_conn(local)
        .await
        .expect("Connection should be handled");

    assert!(
        matches!(
            client.request(p("(def message \"Hello world\")").unwrap()).await,
            Ok(Response { contents, .. }) if contents == Ok(lyric::Form::string("Hello world"))
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
            Ok(Response { contents, .. }) if contents == Ok(lyric::Form::string("Hello world"))
        ),
        "evaluating a function call passing defined argument symbols should return result"
    );
}
