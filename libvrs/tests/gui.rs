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
    // Exercise the real value-picker callback without loading OS services.
    let callbacks = lyric::parse_script(include_str!("../../scripts/vrsjmp.ll"))
        .unwrap()
        .into_iter()
        .filter(|form| matches!(form, Form::List(items)
            if items.first() == Some(&Form::symbol("defn!"))
                && matches!(items.get(1), Some(Form::Symbol(name))
                    if ["choose_items", "function_item", "function_items", "make_item", "on_click"].contains(&name.as_str()))))
        .map(|form| form.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    runtime
        .run(
            Program::from_script(&format!(
                r#"{callbacks}
        (defn! gui_echo (value) "Fixture call" (error "must not execute while browsing"))
        (spawn_srv! :gui_fixture :interface '(gui_echo))
        (bind_srv :gui_fixture)
        (def pending nil)
        (defn! enqueue_input (id owner page)
          (set pending (list id owner page))
          (publish :queued page)
          :ok)
        (defn! finish (status value)
          (send (get pending 1) (list (get pending 0) status value)))
        (defn! input_request (id) pending)
        (defn! finish_input (id value) (finish :ok value) :close)
        (defn! choice_rows (query)
          (def page (get pending 2))
          (apply (eval (get page :get_items))
                 (concat (list (get pending 0)) (get page :args) (list query))))
        (spawn_srv! :vrsjmp :interface '(enqueue_input finish choice_rows on_click))
    "#,
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
    let (client, _) = connect(&runtime).await;
    evaluate(&client, "(bind_srv :vrsjmp)").await;
    (runtime, client)
}

#[tokio::test]
async fn chooser_preserves_values_and_duplicate_labels_after_search() {
    let (runtime, service) = fixture().await;
    let mut queued = service.subscribe(KeywordId::from("queued")).await.unwrap();
    evaluate(&service, "nil").await;
    let cases = [
        (
            r#"(vrsjmp_choose '((:title "Same" :id 1) (:title "Same" :id 2)))"#,
            "Same",
            1,
            r#"(:title "Same" :id 2)"#,
        ),
        ("(vrsjmp_choose '(:red :green))", "green", 0, ":green"),
        (
            "(vrsjmp_choose '(nil false (undefined_function :argument) hello))",
            "undefined_function",
            0,
            "(undefined_function :argument)",
        ),
        ("(vrsjmp_choose '(nil false hello))", "nil", 0, "nil"),
        ("(vrsjmp_choose '(nil false hello))", "false", 0, "false"),
        ("(vrsjmp_choose '(nil false hello))", "hello", 0, "hello"),
        (
            r#"(vrsjmp_choose '("a\n\"b\"" ""))"#,
            "",
            0,
            r#""a\n\"b\"""#,
        ),
        (r#"(vrsjmp_choose '("a" ""))"#, "", 1, r#""""#),
        ("(vrsjmp_choose '(1 ()))", "", 1, "()"),
        (
            r#"(vrsjmp_choose_field '(:title "Example" :url "https://example.org"))"#,
            "url",
            0,
            r#""https://example.org""#,
        ),
        (
            "(vrsjmp_choose_field '(:example/item :nested (undefined_function) :enabled false))",
            "nested",
            0,
            "(undefined_function)",
        ),
        ("(vrsjmp_choose_field '(:empty nil))", "empty", 0, "nil"),
    ];
    for (source, query, index, expected) in cases {
        let (caller, _) = connect(&runtime).await;
        let result = tokio::spawn(async move {
            caller
                .request(Form::from_expr(source).unwrap())
                .await
                .unwrap()
                .contents
        });
        timeout(Duration::from_secs(3), queued.recv())
            .await
            .unwrap()
            .unwrap();
        let Form::List(rows) = evaluate(&service, &format!("(choice_rows {query:?})")).await else {
            panic!("expected chooser rows");
        };
        // Round-trip the selected row through the client, as the GUI does.
        evaluate(&service, &format!("(on_click '{})", rows[index])).await;
        assert_eq!(
            timeout(Duration::from_secs(3), result)
                .await
                .unwrap()
                .unwrap()
                .unwrap(),
            Form::from_expr(expected).unwrap(),
            "{source}"
        );
    }
}

#[tokio::test]
async fn empty_or_malformed_choices_fail_before_requesting_input() {
    let (_runtime, service) = fixture().await;
    for source in [
        "(vrsjmp_choose 1)",
        "(vrsjmp_choose '())",
        "(vrsjmp_choose_field '())",
        "(vrsjmp_choose_field 1)",
        "(vrsjmp_choose_field '(1 2))",
        "(vrsjmp_choose_field '(:key 1 :missing))",
    ] {
        assert_eq!(
            evaluate(&service, &format!("(err? (try {source}))")).await,
            Form::Bool(true),
            "{source}"
        );
    }
}

#[tokio::test]
async fn picker_queues_a_page_and_returns_source_without_executing_it() {
    let (runtime, service) = fixture().await;
    let mut queued = service.subscribe(KeywordId::from("queued")).await.unwrap();
    evaluate(&service, "nil").await;
    let (caller, _) = connect(&runtime).await;
    let result = tokio::spawn(async move {
        caller
            .request(Form::from_expr("(vrsjmp_browse_functions)").unwrap())
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
        :title "Browse service functions" :prompt "Search functions or services…")"#
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
async fn gui_function_browser_uses_shared_metadata_and_returns_a_placeholder_call() {
    let (runtime, service) = fixture().await;
    let mut queued = service.subscribe(KeywordId::from("queued")).await.unwrap();
    evaluate(&service, "nil").await;
    let (caller, _) = connect(&runtime).await;
    let result = tokio::spawn(async move {
        caller
            .request(Form::from_expr("(vrsjmp_browse_functions)").unwrap())
            .await
            .unwrap()
            .contents
    });
    timeout(Duration::from_secs(3), queued.recv())
        .await
        .unwrap()
        .unwrap();
    let Form::List(rows) = evaluate(&service, r#"(choice_rows "gui_echo")"#).await else {
        panic!("expected function rows");
    };
    assert_eq!(rows.len(), 1);
    assert!(rows[0].to_string().contains("Fixture call"));
    assert!(rows[0].to_string().contains(":gui_fixture"));
    // If this invoked gui_echo rather than returning its form, it would fail.
    evaluate(&service, &format!("(on_click '{})", rows[0])).await;
    assert_eq!(
        timeout(Duration::from_secs(3), result)
            .await
            .unwrap()
            .unwrap()
            .unwrap(),
        Form::from_expr("(gui_echo value)").unwrap()
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
            .request(Form::from_expr("(vrsjmp_browse_functions)").unwrap())
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
