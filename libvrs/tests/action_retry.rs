use std::{sync::Arc, time::Duration};
use tokio::time::{sleep, timeout};
use vrs::{Client, Connection, Form, KeywordId, Program, Runtime};

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
    let runtime = Runtime::new("retry-test");
    // Load real palette functions without starting any OS integrations.
    let functions = lyric::parse_script(include_str!("../../scripts/vrsjmp.ll"))
        .unwrap()
        .into_iter()
        .filter(|form| {
            matches!(form, Form::List(items)
            if items.first() == Some(&Form::symbol("defn!")))
        })
        .map(|form| form.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    runtime.run(Program::from_script(&format!(r#"
        (defn! perform_impl (value)
          (publish :action_attempt value)
          (def outcome (get (recv '(:finish _)) 1))
          (match outcome
            (:ok value)
            (:exit '(:exit 7 :stderr "helper failed"))
            (_ (error "attempt failed"))))
        (defn! perform (value)
          "Perform selected action" (interactive :web/page)
          (perform_impl value))
        (spawn_srv! :action_fixture :interface '(perform))
        (defn! feedbin_save (url title) (perform_impl (list url title)))
        (spawn_srv! :feedbin :interface '(feedbin_save))
        (bind_srv :action_fixture)
        {functions}
        (def action_runs '())
        (def pending_inputs '())
        (def things_cache nil)
        (def codex_cache nil)
        (def input_reads 0)
        (def current_page '(:web/page :title "Original" :url "https://example.test/original"))
        (defn! capture_input () (set input_reads (+ input_reads 1)) current_page)
        (defn! active_tab () (slice (capture_input) 1))
        (defn! browser_pages () (list (capture_input)))
        (set_entity_completions :web/page 'browser_pages)
        (defn! set_page (page) (set current_page page))
        (defn! retry_stats () (list input_reads (len action_runs)))
        (defn! favorite_items () '())
        (defn! scheduler_items (query) '())
        (defn! macro_items (query) '())
        (defn! query_items (query) '())
        (spawn_srv! :vrsjmp :interface '(on_click get_items root_page finish_action set_page retry_stats))
    "#)).unwrap()).await.unwrap().join().await.unwrap().status.unwrap();
    let (local, remote) = Connection::pair().unwrap();
    runtime.handle_conn(remote).await.unwrap();
    let client = Arc::new(Client::new(local));
    evaluate(&client, "(bind_srv :vrsjmp)").await;
    (runtime, client)
}

async fn wait_for(client: &Client, source: &str, expected: &str) {
    let expected = Form::from_expr(expected).unwrap();
    let mut last = Form::Nil;
    let result = timeout(Duration::from_secs(7), async {
        loop {
            last = evaluate(client, source).await;
            if last == expected {
                break;
            }
            sleep(Duration::from_millis(5)).await;
        }
    })
    .await;
    assert!(result.is_ok(), "{source}: expected {expected}, got {last}");
}

#[tokio::test]
async fn direct_calls_capture_once_and_retry_against_a_replaced_service() {
    let (_runtime, client) = fixture().await;
    let mut attempts = client
        .subscribe(KeywordId::from("action_attempt"))
        .await
        .unwrap();
    assert_eq!(
        evaluate(
            &client,
            r#"(on_click '(:title "Original item label" :on_click (perform (capture_input))))"#,
        )
        .await,
        Form::keyword("close")
    );
    let original =
        Form::from_expr(r#"(:web/page :title "Original" :url "https://example.test/original")"#)
            .unwrap();
    assert_eq!(
        timeout(Duration::from_secs(2), attempts.recv())
            .await
            .unwrap()
            .unwrap(),
        original
    );
    // The action is still waiting, but the palette can navigate and change input.
    assert_eq!(
        evaluate(
            &client,
            r#"(get (on_click '(:title "Read Later" :on_click (read_later_page))) :title)"#,
        )
        .await,
        Form::string("Read Later")
    );
    evaluate(
        &client,
        r#"(set_page '(:web/page :title "Different" :url "https://example.test/different"))"#,
    )
    .await;
    assert_eq!(
        evaluate(&client, "(retry_stats)").await,
        Form::from_expr("(1 1)").unwrap()
    );
    assert_eq!(
        evaluate(&client, "(get_items 'failed_action_items '() \"\")").await,
        Form::from_expr("()").unwrap()
    );

    evaluate(
        &client,
        "(send (find_srv :action_fixture) '(:finish :exit))",
    )
    .await;
    wait_for(
        &client,
        "(len (get_items 'failed_action_items '() \"\"))",
        "1",
    )
    .await;
    evaluate(
        &client,
        "(def failed (get (get_items 'failed_action_items '() \"Original\") 0))",
    )
    .await;
    assert_eq!(
        evaluate(&client, "(get failed :title)").await,
        Form::string("Original item label")
    );
    assert_eq!(
        evaluate(&client, "(get failed :aside)").await,
        Form::string("Failed")
    );
    let subtitle = evaluate(&client, "(get failed :subtitle)")
        .await
        .to_string();
    assert!(
        subtitle.contains("Command failed: helper failed"),
        "{subtitle}"
    );
    assert!(subtitle.contains("(perform '(:web/page"), "{subtitle}");
    assert!(subtitle.contains("example.test/original"), "{subtitle}");
    assert!(!subtitle.contains("example.test/different"));
    assert_eq!(
        evaluate(
            &client,
            "(get (get (get_items 'root_items '() \"Original\") 0) :aside)"
        )
        .await,
        Form::string("Failed")
    );

    // Same symbol, new service PID. A captured PID would fail this test.
    evaluate(
        &client,
        r#"(begin
      (defn! perform (value) (publish :action_attempt value) value)
      (spawn_srv! :action_fixture :interface '(perform)))"#,
    )
    .await;
    assert_eq!(
        evaluate(&client, "(on_click failed)").await,
        Form::keyword("refresh")
    );
    assert_eq!(
        evaluate(&client, "(on_click failed)").await,
        Form::keyword("refresh")
    );
    assert_eq!(
        timeout(Duration::from_secs(2), attempts.recv())
            .await
            .unwrap()
            .unwrap(),
        original
    );
    wait_for(&client, "(retry_stats)", "(1 0)").await;
    assert!(timeout(Duration::from_millis(30), attempts.recv())
        .await
        .is_err());
}

#[tokio::test]
async fn save_shortcut_and_interactive_choices_share_the_captured_action_path() {
    let (_runtime, client) = fixture().await;
    let mut attempts = client
        .subscribe(KeywordId::from("action_attempt"))
        .await
        .unwrap();
    evaluate(
        &client,
        r#"(def save (get (get_items 'command_items '() "Save to Read Later") 0))"#,
    )
    .await;
    assert_eq!(
        evaluate(&client, "(get save :on_click)").await,
        Form::from_expr("(save_page (active_tab))").unwrap()
    );
    assert_eq!(
        evaluate(&client, "(retry_stats)").await,
        Form::from_expr("(0 0)").unwrap()
    );
    assert_eq!(
        evaluate(&client, "(on_click save)").await,
        Form::keyword("close")
    );
    let saved = Form::from_expr(r#"("https://example.test/original" "Original")"#).unwrap();
    assert_eq!(
        timeout(Duration::from_secs(2), attempts.recv())
            .await
            .unwrap()
            .unwrap(),
        saved
    );
    evaluate(
        &client,
        r#"(set_page '(:web/page :title "Other" :url "https://example.test/other"))"#,
    )
    .await;
    evaluate(&client, "(send (find_srv :feedbin) '(:finish :error))").await;
    wait_for(
        &client,
        "(len (get_items 'failed_action_items '() \"\"))",
        "1",
    )
    .await;
    evaluate(
        &client,
        "(def failed (get (get_items 'failed_action_items '() \"\") 0))",
    )
    .await;
    assert_eq!(
        evaluate(&client, "(get failed :title)").await,
        Form::string("Save to Read Later")
    );
    assert!(evaluate(&client, "(get failed :subtitle)")
        .await
        .to_string()
        .contains("(save_page '(:title"));
    assert!(evaluate(&client, "(get failed :subtitle)")
        .await
        .to_string()
        .contains("attempt failed"));
    evaluate(&client, "(on_click failed)").await;
    assert_eq!(
        timeout(Duration::from_secs(2), attempts.recv())
            .await
            .unwrap()
            .unwrap(),
        saved
    );
    evaluate(&client, "(send (find_srv :feedbin) '(:finish :ok))").await;
    wait_for(&client, "(retry_stats)", "(1 0)").await;

    evaluate(&client, r#"(def page (on_click '(:title "Save to Read Later" :on_click (call_interactively 'save_page))))"#).await;
    assert_eq!(
        evaluate(&client, "(get page :get_items)").await,
        Form::symbol("call_items")
    );
    assert_eq!(
        evaluate(&client, "(retry_stats)").await,
        Form::from_expr("(1 0)").unwrap()
    );
    evaluate(
        &client,
        "(def choices (get_items (get page :get_items) (get page :args) \"\"))",
    )
    .await;
    evaluate(&client, "(on_click (get choices 0))").await;
    assert_eq!(
        timeout(Duration::from_secs(2), attempts.recv())
            .await
            .unwrap()
            .unwrap(),
        Form::from_expr(r#"("https://example.test/other" "Other")"#).unwrap()
    );
    evaluate(&client, "(send (find_srv :feedbin) '(:finish :ok))").await;
    wait_for(&client, "(retry_stats)", "(2 0)").await;
}

#[tokio::test]
async fn argument_errors_and_compound_forms_do_not_create_retry_records() {
    let (_runtime, client) = fixture().await;
    assert_eq!(
        evaluate(
            &client,
            r#"(err? (try (on_click '(:title "Save to Read Later"
                :on_click (save_page (error "context unavailable"))))))"#,
        )
        .await,
        Form::from_expr("true").unwrap()
    );
    assert_eq!(
        evaluate(&client, "(retry_stats)").await,
        Form::from_expr("(0 0)").unwrap()
    );

    // A begin follows normal evaluation, without snapshotting inner calls.
    assert_eq!(
        evaluate(
            &client,
            r#"(err? (try (on_click '(:title "Compound action"
                :on_click (begin (capture_input) (error "compound failed"))))))"#,
        )
        .await,
        Form::from_expr("true").unwrap()
    );
    assert_eq!(
        evaluate(&client, "(retry_stats)").await,
        Form::from_expr("(1 0)").unwrap()
    );
}

#[tokio::test]
async fn timeouts_keep_their_error_and_dismiss_does_not_replay_literal_values() {
    let (_runtime, client) = fixture().await;
    let mut attempts = client
        .subscribe(KeywordId::from("action_attempt"))
        .await
        .unwrap();
    evaluate(&client, r#"(on_click '(:title "Literal values" :on_click (perform '(undefined_function false nil))))"#).await;
    assert_eq!(
        attempts.recv().await.unwrap(),
        Form::from_expr("(undefined_function false nil)").unwrap()
    );
    wait_for(
        &client,
        "(len (get_items 'failed_action_items '() \"\"))",
        "1",
    )
    .await;
    evaluate(
        &client,
        "(def failed (get (get_items 'failed_action_items '() \"\") 0))",
    )
    .await;
    let subtitle = evaluate(&client, "(get failed :subtitle)")
        .await
        .to_string();
    assert!(subtitle.contains("timed out after 5 seconds"), "{subtitle}");
    assert!(
        subtitle.contains("(perform '(undefined_function false nil))"),
        "{subtitle}"
    );
    assert_eq!(
        evaluate(&client, "(on_click (get (get failed :actions) 1))").await,
        Form::keyword("refresh")
    );
    assert_eq!(
        evaluate(&client, "(retry_stats)").await,
        Form::from_expr("(0 0)").unwrap()
    );
    evaluate(&client, "(on_click failed)").await;
    assert_eq!(
        evaluate(&client, "(retry_stats)").await,
        Form::from_expr("(0 0)").unwrap()
    );
    // Completion of the old remote call doesn't bring a dismissed row back.
    evaluate(&client, "(send (find_srv :action_fixture) '(:finish :ok))").await;
    assert_eq!(
        evaluate(&client, "(retry_stats)").await,
        Form::from_expr("(0 0)").unwrap()
    );
}
