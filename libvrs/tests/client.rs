//! E2E tests for Runtime and Client
use assert_matches::assert_matches;
use lyric::Form;
use vrs::{Client, Connection, Response, Runtime};

#[tokio::test]
async fn request_response() {
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
async fn request_response_multi() {
    use lyric::parse as p;

    let runtime = Runtime::new();
    let (local, remote) = Connection::pair().unwrap();
    let client = Client::new(remote);

    runtime
        .handle_conn(local)
        .await
        .expect("Connection should be handled");

    assert_matches!(
        client.request(p("(def message \"Hello world\")").unwrap()).await,
        Ok(Response { contents, .. }) if contents == Ok(lyric::Form::string("Hello world")),
        "defining a message binding should return its value"
    );
    assert_matches!(
        client
            .request(p("(def echo (lambda (x) x))").unwrap())
            .await,
        Ok(Response { .. }),
        "defining a echo binding is successful"
    );
    assert_matches!(
        client.request(p("(echo message)").unwrap()).await,
        Ok(Response { contents, .. }) if contents == Ok(lyric::Form::string("Hello world")),
        "evaluating a function call passing defined argument symbols should return result"
    );
}

#[tokio::test]
async fn request_response_parallel() {
    let (local, remote) = Connection::pair().unwrap();

    let rt = Runtime::new();
    let _ = rt.handle_conn(remote).await.unwrap();

    let client = Client::new(local);

    let req1 = client.request(lyric::parse("(+ 0 1)").unwrap());
    let req2 = client.request(lyric::parse("(+ 1 1)").unwrap());
    let req3 = client.request(lyric::parse("(+ 2 1)").unwrap());

    assert_matches!(
        tokio::try_join!(req2, req1, req3).unwrap(),
        (
            Response {
                contents: Ok(Form::Int(2)),
                ..
            },
            Response {
                contents: Ok(Form::Int(1)),
                ..
            },
            Response {
                contents: Ok(Form::Int(3)),
                ..
            },
        )
    );
}
