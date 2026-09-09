use super::*;
use vrs::{Program, Runtime};

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
        (spawn_srv! :vrsjmp :interface '(root_page get_items on_click click_count))
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
