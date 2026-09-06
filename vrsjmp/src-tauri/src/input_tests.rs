//! GUI input protocol tests use local functions and an embedded runtime.
use super::*;
use vrs::{ProcessHandle, ProcessResult, Program, Runtime, Val};

async fn fixture() -> (Runtime, vrs::Client) {
    let runtime = Runtime::new("input-test");
    let mut body = vec![Val::symbol("begin")];
    body.extend(lyric::parse_script(include_str!("../../../scripts/vrsjmp.ll")).unwrap()
        .into_iter().filter(|form| matches!(form, Form::List(values) if
            values.first() == Some(&Form::symbol("defn!")) || values.first() == Some(&Form::symbol("def"))))
        .map(Val::from));
    body.extend(
        lyric::parse_script(
            r#"
        (defn! example (item destination)
          "Example function" (interactive :choice :choice)
          (error "The picker must never execute this function"))
        (defn! choices () '((:choice :value 7 :title "Choice seven")))
        (set_entity_completions :choice 'choices)
        (spawn_srv! :example :interface '(example choices))
        (bind_srv :example)
        (spawn_srv! :vrsjmp :interface '(get_items on_click enqueue_input))
    "#,
        )
        .unwrap()
        .into_iter()
        .map(Val::from),
    );
    runtime
        .run(Program::from_val(Val::List(body)).unwrap())
        .await
        .unwrap()
        .join()
        .await
        .unwrap()
        .status
        .unwrap();
    let (local, remote) = Connection::pair().unwrap();
    runtime.handle_conn(remote).await.unwrap();
    (runtime, vrs::Client::new(local))
}

async fn evaluate(client: &vrs::Client, form: Form) -> Form {
    tokio::time::timeout(Duration::from_secs(3), client.request(form))
        .await
        .unwrap()
        .unwrap()
        .contents
        .unwrap()
}

async fn begin(client: &vrs::Client) -> protocol::Page {
    page(
        evaluate(
            client,
            protocol::action_request("(:on_click (begin_interaction))").unwrap(),
        )
        .await,
    )
}

fn page(value: Form) -> protocol::Page {
    let protocol::Action::PushPage { page } = protocol::action(value).unwrap() else {
        panic!("expected page")
    };
    page
}

async fn items(client: &vrs::Client, page: &protocol::Page, query: &str) -> Vec<protocol::Item> {
    protocol::items(
        evaluate(
            client,
            protocol::query_request(&page.get_items, &page.args, query).unwrap(),
        )
        .await,
    )
    .unwrap()
}

async fn click(client: &vrs::Client, command: &str) -> Form {
    evaluate(client, protocol::action_request(command).unwrap()).await
}

async fn start_picker(
    runtime: &Runtime,
) -> (
    Arc<vrs::Client>,
    ProcessHandle,
    tokio::task::JoinHandle<Result<Response>>,
) {
    let (local, remote) = Connection::pair().unwrap();
    let process = runtime.handle_conn(remote).await.unwrap();
    let owner = Arc::new(vrs::Client::new(local));
    let client = owner.clone();
    let result = tokio::spawn(async move {
        client
            .request(Form::from_expr("(pick_call)").unwrap())
            .await
            .map_err(anyhow::Error::from)
    });
    (owner, process, result)
}

#[tokio::test]
async fn picker_returns_templates_and_filled_source_without_execution() {
    let (runtime, gui) = fixture().await;
    let mut wakeups = gui.subscribe(vrs::KeywordId::from("vrsjmp")).await.unwrap();
    evaluate(&gui, Form::Nil).await; // subscription is installed before creating work
    let (_owner, _process, result) = start_picker(&runtime).await;
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(3), wakeups.recv())
            .await
            .unwrap()
            .unwrap(),
        Form::keyword("show")
    );
    let root = begin(&gui).await;
    assert!(root.on_cancel.is_some());
    let all = items(&gui, &root, "").await;
    assert_eq!(
        all.len(),
        2,
        "only the two bound methods appear, not local helpers"
    );
    assert!(items(&gui, &root, "fill_call").await.is_empty());
    let functions = items(&gui, &root, "example").await;
    let function = functions
        .iter()
        .find(|row| row.title == "(example item destination)")
        .unwrap();
    assert_eq!(
        click(&gui, &function.on_click).await,
        Form::keyword("close")
    );
    assert_eq!(
        result.await.unwrap().unwrap().contents.unwrap(),
        Form::from_expr("(example item destination)").unwrap()
    );

    let (_owner, _process, result) = start_picker(&runtime).await;
    tokio::time::timeout(Duration::from_secs(3), wakeups.recv())
        .await
        .unwrap()
        .unwrap();
    let root = begin(&gui).await;
    let functions = items(&gui, &root, "example").await;
    let function = functions
        .iter()
        .find(|row| row.title == "(example item destination)")
        .unwrap();
    let fill = function
        .actions
        .iter()
        .find(|action| action.title == "Fill arguments")
        .unwrap();
    let first = page(click(&gui, &fill.on_click).await);
    let rows = items(&gui, &first, "").await;
    assert_eq!(
        rows.len(),
        1,
        "argument pages contain only completion choices"
    );
    let second = page(click(&gui, &rows[0].on_click).await);
    let rows = items(&gui, &second, "seven").await;
    assert_eq!(rows.len(), 1);
    assert_eq!(click(&gui, &rows[0].on_click).await, Form::keyword("close"));
    assert_eq!(
        result.await.unwrap().unwrap().contents.unwrap(),
        Form::from_expr("(example '(:choice :value 7 :title \"Choice seven\") '(:choice :value 7 :title \"Choice seven\"))").unwrap()
    );
}

#[tokio::test]
async fn pending_work_survives_missed_wakeups_and_cancel_returns_an_error() {
    let (runtime, gui) = fixture().await;
    // No subscriber: the request still appears through ordinary queries.
    let (_owner, _process, result) = start_picker(&runtime).await;
    let pending = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let rows = protocol::items(
                evaluate(
                    &gui,
                    protocol::query_request("pending_input_items", "()", "").unwrap(),
                )
                .await,
            )
            .unwrap();
            if !rows.is_empty() {
                break rows;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let root = page(click(&gui, &pending[0].on_click).await);
    click(&gui, root.on_cancel.as_ref().unwrap()).await;
    assert!(
        result.await.unwrap().unwrap().contents.is_err(),
        "cancel must not replace source with nil"
    );
    assert!(items(
        &gui,
        &protocol::Page {
            get_items: "pending_input_items".into(),
            args: "()".into(),
            title: "Pending".into(),
            prompt: "Search".into(),
            debounce_ms: 0,
            on_cancel: None
        },
        ""
    )
    .await
    .is_empty());
}

#[tokio::test]
async fn disconnect_cancels_the_waiting_evaluation_and_removes_its_request() {
    let (runtime, gui) = fixture().await;
    let mut wakeups = gui.subscribe(vrs::KeywordId::from("vrsjmp")).await.unwrap();
    evaluate(&gui, Form::Nil).await;
    let (owner, process, result) = start_picker(&runtime).await;
    tokio::time::timeout(Duration::from_secs(3), wakeups.recv())
        .await
        .unwrap()
        .unwrap();
    owner.shutdown().await;
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(3), process.join())
            .await
            .unwrap()
            .unwrap()
            .status
            .unwrap(),
        ProcessResult::Cancelled
    );
    let _ = result.await; // the disconnected request task ends too
    let rows = protocol::items(
        evaluate(
            &gui,
            protocol::query_request("pending_input_items", "()", "").unwrap(),
        )
        .await,
    )
    .unwrap();
    assert!(rows.is_empty());
}

#[tokio::test]
async fn gui_bridge_uses_one_connection_for_queries_and_wakeups_and_resubscribes() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let runtime = Arc::new(Runtime::new("bridge-test"));
    let path = std::env::temp_dir().join(format!(
        "vrs-gui-{}-{}.sock",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let listener = tokio::net::UnixListener::bind(&path).unwrap();
    let accepted = Arc::new(AtomicUsize::new(0));
    let server = {
        let runtime = runtime.clone();
        let accepted = accepted.clone();
        tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                accepted.fetch_add(1, Ordering::SeqCst);
                runtime.handle_conn(Connection::new(stream)).await.unwrap();
            }
        })
    };
    let (events, mut notifications) = mpsc::unbounded_channel();
    let mut bridge = Client::new(path.clone());
    bridge
        .start(move |event| {
            let _ = events.send(event.to_string());
        })
        .unwrap();
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(3), notifications.recv())
            .await
            .unwrap()
            .unwrap(),
        "vrs-connected"
    );
    // Observe a wakeup before the pending request can finish. Both use the
    // GUI's one socket, and the response arrives only after a separate process replies.
    let waiting = bridge.request(
        Form::from_expr("(begin (register :listener) (publish :vrsjmp :show) (recv))").unwrap(),
    );
    let (response, _) = tokio::join!(waiting, async {
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(3), notifications.recv())
                .await
                .unwrap()
                .unwrap(),
            "show-palette"
        );
        runtime
            .run(Program::from_expr("(send (find_srv :listener) :answer)").unwrap())
            .await
            .unwrap()
            .join()
            .await
            .unwrap()
            .status
            .unwrap();
    });
    assert_eq!(response.unwrap().contents.unwrap(), Form::keyword("answer"));
    assert_eq!(accepted.load(Ordering::SeqCst), 1);
    let _ = bridge
        .request(Form::from_expr("(kill (self))").unwrap())
        .await;
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(3), notifications.recv())
            .await
            .unwrap()
            .unwrap(),
        "vrs-connected"
    );
    assert_eq!(accepted.load(Ordering::SeqCst), 2);
    bridge
        .request(Form::from_expr("(publish :vrsjmp :show)").unwrap())
        .await
        .unwrap();
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(3), notifications.recv())
            .await
            .unwrap()
            .unwrap(),
        "show-palette"
    );
    server.abort();
    std::fs::remove_file(path).unwrap();
}
