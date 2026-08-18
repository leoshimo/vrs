use std::{sync::Arc, time::Duration};
use tokio::time::timeout;
use vrs::{Client, Connection, Form, KeywordId, ProcessHandle, ProcessResult, Program, Runtime};

async fn connect(runtime: &Runtime) -> (Arc<Client>, ProcessHandle) {
    let (local, remote) = Connection::pair().unwrap();
    let process = runtime.handle_conn(remote).await.unwrap();
    (Arc::new(Client::new(local)), process)
}

async fn evaluate(client: &Client, source: &str) -> Form {
    timeout(
        Duration::from_secs(3),
        client.request(Form::from_expr(source).unwrap()),
    )
    .await
    .unwrap()
    .unwrap()
    .contents
    .unwrap()
}

async fn fixture() -> (Runtime, Arc<Client>) {
    let runtime = Runtime::new("gui-test");
    runtime
        .run(
            Program::from_script(
                r#"
        (def pending nil)
        (defn! enqueue_input (id owner page)
          (set pending (list id owner))
          (publish :queued page)
          :ok)
        (defn! finish (status value)
          (send (get pending 1) (list (get pending 0) status value)))
        (spawn_srv! :vrsjmp :interface '(enqueue_input finish))
    "#,
            )
            .unwrap(),
        )
        .await
        .unwrap()
        .join()
        .await
        .unwrap()
        .status
        .unwrap();
    let (client, _) = connect(&runtime).await;
    evaluate(&client, "(bind_srv :vrsjmp)").await;
    (runtime, client)
}

#[tokio::test]
async fn picker_queues_a_page_and_returns_source_without_executing_it() {
    let (runtime, service) = fixture().await;
    let mut queued = service.subscribe(KeywordId::from("queued")).await.unwrap();
    evaluate(&service, "nil").await;
    let (caller, _) = connect(&runtime).await;
    let result = tokio::spawn(async move {
        caller
            .request(Form::from_expr("(pick_call)").unwrap())
            .await
            .unwrap()
            .contents
    });
    let page = timeout(Duration::from_secs(3), queued.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        page,
        Form::from_expr(
            r#"(:push_page :get_items function_items :args ()
        :title "Insert a call" :prompt "Find a service function…")"#
        )
        .unwrap()
    );
    evaluate(&service, "(finish :pending nil)").await;
    evaluate(&service, "(finish :ok '(undefined_function :argument))").await;
    assert_eq!(
        timeout(Duration::from_secs(3), result)
            .await
            .unwrap()
            .unwrap()
            .unwrap(),
        Form::from_expr("(undefined_function :argument)").unwrap()
    );
}

#[tokio::test]
async fn cancelled_input_is_an_evaluation_error() {
    let (runtime, service) = fixture().await;
    let mut queued = service.subscribe(KeywordId::from("queued")).await.unwrap();
    evaluate(&service, "nil").await;
    let (caller, _) = connect(&runtime).await;
    let result = tokio::spawn(async move {
        caller
            .request(Form::from_expr("(request_input '(:push_page :get_items choices))").unwrap())
            .await
            .unwrap()
            .contents
    });
    timeout(Duration::from_secs(3), queued.recv())
        .await
        .unwrap()
        .unwrap();
    evaluate(&service, "(finish :cancelled nil)").await;
    let error = timeout(Duration::from_secs(3), result)
        .await
        .unwrap()
        .unwrap()
        .unwrap_err();
    assert!(error.to_string().contains("GUI input cancelled"));
}

#[tokio::test]
async fn disconnect_ends_a_process_waiting_for_gui_input() {
    let (runtime, service) = fixture().await;
    let mut queued = service.subscribe(KeywordId::from("queued")).await.unwrap();
    evaluate(&service, "nil").await;
    let (caller, process) = connect(&runtime).await;
    let request_client = caller.clone();
    let result = tokio::spawn(async move {
        request_client
            .request(Form::from_expr("(pick_call)").unwrap())
            .await
    });
    timeout(Duration::from_secs(3), queued.recv())
        .await
        .unwrap()
        .unwrap();
    caller.shutdown().await;
    assert_eq!(
        timeout(Duration::from_secs(3), process.join())
            .await
            .unwrap()
            .unwrap()
            .status
            .unwrap(),
        ProcessResult::Cancelled
    );
    assert!(timeout(Duration::from_secs(3), result)
        .await
        .unwrap()
        .unwrap()
        .is_err());
}
