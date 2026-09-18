use super::*;
use vrs::{KeywordId, Program, Runtime};

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
    bridge
        .request(Form::from_expr("(publish :vrsjmp :config_changed)").unwrap())
        .await
        .unwrap();
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(3), notifications.recv())
            .await
            .unwrap()
            .unwrap(),
        "vrs-config-changed"
    );
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

    runtime
        .run(
            Program::from_script(
                r#"
        (defn! get_items (callback args query)
          (if (eq? query "fail") (error "query failed"))
          (list (list :title query :on_click :close)))
        (def clicks 0)
        (defn! root_page ()
          '(:push_page :get_items items :args ("request-1" (:object :id 7))
            :title "Choose" :prompt "Find an item"
            :on_cancel (cancel_input "request-1")))
        (defn! click_count () clicks)
        (defn! on_click (item)
          (set clicks (+ clicks 1))
          (get item :on_click))
        (defn! on_click_wait (item receiver) (on_click item))
        (spawn_srv! :vrsjmp :interface '(root_page get_items on_click on_click_wait click_count))
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
    let response = bridge.request(protocol::root_request()).await.unwrap();
    let protocol::Action::PushPage { page } = protocol::action(response.contents.unwrap()).unwrap()
    else {
        panic!("root request must return a page")
    };
    assert_eq!(page.get_items, "items");
    assert_eq!(page.args, "(\"request-1\" (:object :id 7))");
    assert_eq!(
        page.on_cancel.as_deref(),
        Some("(:on_click (cancel_input \"request-1\"))")
    );
    assert_eq!(
        bridge
            .request(protocol::service_request("click_count", vec![]))
            .await
            .unwrap()
            .contents
            .unwrap(),
        Form::Int(0)
    );
    let failed = bridge
        .request(protocol::query_request("items", "()", "fail").unwrap())
        .await
        .unwrap();
    assert!(failed.contents.is_err());
    let query = "quotes \" newline\n) (undefined_function)";
    let response = bridge
        .request(protocol::query_request("items", "()", query).unwrap())
        .await
        .unwrap();
    let items = protocol::items(response.contents.unwrap()).unwrap();
    assert_eq!(items[0].title, query);
    let response = bridge
        .request(protocol::action_request(&items[0].on_click).unwrap())
        .await
        .unwrap();
    assert_eq!(
        protocol::action(response.contents.unwrap()).unwrap(),
        protocol::Action::Close
    );
    assert_eq!(accepted.load(Ordering::SeqCst), 2);
    server.abort();
    std::fs::remove_file(path).unwrap();
}

#[tokio::test]
async fn action_request_waits_for_execution_and_preserves_failures_for_retry() {
    let runtime = Runtime::new("action-completion-test");
    let functions = lyric::parse_script(include_str!("../../../scripts/vrsjmp.ll"))
        .unwrap().into_iter().filter(|form| {
            matches!(form, Form::List(items) if items.first() == Some(&Form::symbol("defn!")))
        }).map(|form| form.to_string()).collect::<Vec<_>>().join("\n");
    runtime
        .run(
            Program::from_script(&format!(
                r#"
        (defn! perform (value)
          (publish :attempt value)
          (if (eq? (recv) :ok) :ok (error "API unavailable")))
        (defn! quick () :ok)
        (spawn_srv! :worker :interface '(perform quick))
        (bind_srv :worker)
        {functions}
        (def action_runs '())
        (spawn_srv! :vrsjmp :interface '(on_click_wait finish_action get_items))
    "#
            ))
            .unwrap(),
        )
        .await
        .unwrap()
        .join()
        .await
        .unwrap()
        .status
        .unwrap();
    let (local, remote) = Connection::pair().unwrap();
    runtime.handle_conn(remote).await.unwrap();
    let client = Arc::new(vrs::Client::new(local));
    let (local, remote) = Connection::pair().unwrap();
    runtime.handle_conn(remote).await.unwrap();
    let control = vrs::Client::new(local);
    let mut attempts = client.subscribe(KeywordId::from("attempt")).await.unwrap();
    let request =
        protocol::action_request(r#"(:title "Save" :on_click (perform "captured"))"#).unwrap();
    let mut waiting = {
        let client = client.clone();
        tokio::spawn(async move { client.request(request).await.unwrap().contents.unwrap() })
    };
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(2), attempts.recv())
            .await
            .unwrap()
            .unwrap(),
        Form::string("captured")
    );
    assert!(
        tokio::time::timeout(Duration::from_millis(30), &mut waiting)
            .await
            .is_err()
    );
    // The palette remains responsive while the request process awaits its action.
    let query = protocol::query_request("failed_action_items", "()", "").unwrap();
    assert_eq!(
        control
            .request(query.clone())
            .await
            .unwrap()
            .contents
            .unwrap(),
        Form::List(vec![])
    );
    control
        .request(Form::from_expr("(send (find_srv :worker) :fail)").unwrap())
        .await
        .unwrap();
    let result = tokio::time::timeout(Duration::from_secs(2), waiting)
        .await
        .unwrap()
        .unwrap();
    assert!(protocol::action(result)
        .unwrap_err()
        .to_string()
        .contains("API unavailable"));
    let failed = protocol::items(
        control
            .request(query.clone())
            .await
            .unwrap()
            .contents
            .unwrap(),
    )
    .unwrap();
    assert_eq!(failed.len(), 1);
    let request = protocol::action_request(&failed[0].on_click).unwrap();
    let waiting = {
        let client = client.clone();
        tokio::spawn(async move { client.request(request).await.unwrap().contents.unwrap() })
    };
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(2), attempts.recv())
            .await
            .unwrap()
            .unwrap(),
        Form::string("captured")
    );
    control
        .request(Form::from_expr("(send (find_srv :worker) :ok)").unwrap())
        .await
        .unwrap();
    let result = tokio::time::timeout(Duration::from_secs(2), waiting)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(protocol::action(result).unwrap(), protocol::Action::Refresh);
    assert_eq!(
        control.request(query).await.unwrap().contents.unwrap(),
        Form::List(vec![])
    );
    // Fast actions can finish before the caller begins receiving their result.
    let request = protocol::action_request(r#"(:title "Fast" :on_click (quick))"#).unwrap();
    let result = tokio::time::timeout(Duration::from_secs(2), client.request(request))
        .await
        .unwrap()
        .unwrap()
        .contents
        .unwrap();
    assert_eq!(protocol::action(result).unwrap(), protocol::Action::Close);
}
