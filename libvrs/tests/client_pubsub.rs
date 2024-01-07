//! Tests for [Client] Pubsub API

use vrs::{Client, Connection, Form, KeywordId, Program, Runtime};

/// Test Client::subscribe between client and runtime service process
#[tokio::test]
async fn client_pubsub_from_service() {
    let rt = Runtime::new();

    // counter service
    let prog = Program::from_expr(
        r#"(begin
        (def count 0)
        (defn increment (n)
            (set count (+ count n))
            (publish :count count))
        (srv :name :counter :exports '(increment)))
        "#,
    )
    .unwrap();
    rt.run(prog).await.unwrap();

    let (local, remote) = Connection::pair().unwrap();
    let client = Client::new(local);
    rt.handle_conn(remote).await.unwrap();

    // subscribe + trigger pusub update from same client
    let mut sub = client.subscribe(KeywordId::from("count")).await.unwrap();
    client
        .request(
            Form::from_expr(
                r#"(begin
                    (bind-srv :counter)
                    (increment 1)
                    (increment 10)
                    (increment 31))
                    "#,
            )
            .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        sub.recv().await.unwrap(),
        Form::Int(1),
        "should publish 1 after (increment 1)"
    );
    assert_eq!(
        sub.recv().await.unwrap(),
        Form::Int(11),
        "should publish 11 after (increment 11)"
    );
    assert_eq!(
        sub.recv().await.unwrap(),
        Form::Int(42),
        "should publish 42 after (increment 31)"
    );
}

/// Test Client::subscribe between two clients
#[tokio::test]
async fn client_pubsub_from_another_client() {
    let rt = Runtime::new();

    let client1 = {
        let (local, remote) = Connection::pair().unwrap();
        rt.handle_conn(remote).await.unwrap();
        Client::new(local)
    };

    let client2 = {
        let (local, remote) = Connection::pair().unwrap();
        rt.handle_conn(remote).await.unwrap();
        Client::new(local)
    };

    let mut sub = client1
        .subscribe(KeywordId::from("my_topic"))
        .await
        .unwrap();

    client2
        .request(Form::from_expr("(publish :my_topic :hello)").unwrap())
        .await
        .unwrap();

    assert_eq!(
        sub.recv().await.unwrap(),
        Form::keyword("hello"),
        "subscription from client1 should observe published value from client2"
    );
}

/// Test Client::subscribe between two clients and a service
#[tokio::test]
async fn client_pubsub_from_another_client_via_service() {
    let rt = Runtime::new();

    // counter service
    let prog = Program::from_expr(
        r#"(begin
        (def count 0)
        (defn increment (n)
            (set count (+ count n))
            (publish :count count))
        (srv :name :counter :exports '(increment)))
        "#,
    )
    .unwrap();
    rt.run(prog).await.unwrap();

    let client1 = {
        let (local, remote) = Connection::pair().unwrap();
        rt.handle_conn(remote).await.unwrap();
        Client::new(local)
    };

    let client2 = {
        let (local, remote) = Connection::pair().unwrap();
        rt.handle_conn(remote).await.unwrap();
        Client::new(local)
    };

    // Subscribe in client1
    let mut sub = client1.subscribe(KeywordId::from("count")).await.unwrap();

    // Trigger counter service's pubsub topic via client2
    client2
        .request(
            Form::from_expr(
                r#"(begin
                    (bind-srv :counter)
                    (increment 1)
                    (increment 10)
                    (increment 31))
                    "#,
            )
            .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(
        sub.recv().await.unwrap(),
        Form::Int(1),
        "subscription from client1 should receive pubsub updates from service topic that was updated via another client"
    );
    assert_eq!(
        sub.recv().await.unwrap(),
        Form::Int(11),
        "subscription from client1 should receive pubsub updates from service topic that was updated via another client"
    );
    assert_eq!(
        sub.recv().await.unwrap(),
        Form::Int(42),
        "subscription from client1 should receive pubsub updates from service topic that was updated via another client"
    );
}

// TODO: Test that Client::request (subscribe :my_topic) is different from Client::subscribe(my_topic) (former is observable via ls-msgs + subscribes at process level)
