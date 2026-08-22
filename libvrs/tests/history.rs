//! History through real client connections and service dispatch.
use std::time::Duration;

use lyric::{parse, Form};
use tokio::time::timeout;
use vrs::{Client, Connection, ProcessHandle, ProcessResult, Program, Runtime};

async fn connect(runtime: &Runtime) -> (Client, ProcessHandle) {
    let (client, daemon) = Connection::pair().unwrap();
    let process = runtime.handle_conn(daemon).await.unwrap();
    (Client::new(client), process)
}

async fn request(client: &Client, form: Form) -> vrs::Response {
    timeout(Duration::from_secs(5), client.request(form))
        .await
        .expect("history request timed out")
        .unwrap()
}

async fn eval(client: &Client, source: &str) -> Form {
    request(client, parse(source).unwrap())
        .await
        .contents
        .unwrap()
}

async fn setup(runtime: &Runtime, source: &str) {
    let process = runtime
        .run(Program::from_script(source).unwrap())
        .await
        .unwrap();
    let exit = timeout(Duration::from_secs(5), process.join())
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(exit.status.unwrap(), ProcessResult::Done(_)));
}

const COUNTER: &str = r#"
    (def count 0)
    (defn! add_count (amount) (set count (+ count amount)))
    (defn! get_count () count)
    (spawn_srv! :counter :interface '(add_count get_count))
"#;

#[tokio::test]
async fn client_and_service_histories_capture_different_expressions() {
    let runtime = Runtime::new("test");
    setup(&runtime, COUNTER).await;
    let (client, process) = connect(&runtime).await;
    assert_eq!(eval(&client, "(history)").await, parse("()").unwrap());
    eval(&client, "(bind_srv :counter)").await;
    assert_eq!(eval(&client, "(add_count (+ 1 1))").await, Form::Int(2));
    let expected = parse("((bind_srv :counter) (add_count (+ 1 1)))").unwrap();
    for _ in 0..3 {
        assert_eq!(eval(&client, "(history)").await, expected);
        assert_eq!(
            eval(&client, "(history :counter)").await,
            parse("((add_count 2))").unwrap()
        );
        assert_eq!(
            eval(&client, "(history (find_srv :counter))").await,
            parse("((add_count 2))").unwrap()
        );
    }
    let (other, _) = connect(&runtime).await;
    assert_eq!(
        eval(&other, &format!("(history (pid {}))", process.id().inner())).await,
        expected
    );
    assert_eq!(eval(&other, "(history)").await, parse("()").unwrap());
    assert_eq!(
        eval(&other, "(history (pid 99999))").await,
        parse("()").unwrap()
    );
    assert!(request(&other, parse("(history :missing)").unwrap())
        .await
        .contents
        .is_err());
    for source in ["(history 5)", "(history nil)", "(history :counter 5)"] {
        assert!(request(&other, parse(source).unwrap())
            .await
            .contents
            .is_err());
    }
}

#[tokio::test]
async fn source_requests_are_unwrapped_without_changing_execution() {
    let runtime = Runtime::new("test");
    let (client, _) = connect(&runtime).await;
    let single = lyric::source::request("(+ 1 2)", "editor.ll", 12, 4);
    assert_eq!(
        request(&client, single).await.contents.unwrap(),
        Form::Int(3)
    );
    let multiple = lyric::source::request("(def n 4)\n(+ n 1)", "editor.ll", 20, 1);
    assert_eq!(
        request(&client, multiple).await.contents.unwrap(),
        Form::Int(5)
    );
    let expected = parse("((+ 1 2) (begin (def n 4) (+ n 1)))").unwrap();
    let query = lyric::source::request("# editor query\n(history)", "editor.ll", 30, 1);
    assert_eq!(request(&client, query).await.contents.unwrap(), expected);
    assert_eq!(eval(&client, "(history)").await, expected);

    // One request is one entry; a compound submission using history is kept.
    let compound = parse("(begin (history) 42)").unwrap();
    request(&client, compound.clone()).await.contents.unwrap();
    let malformed = lyric::source::request("(+ 1", "broken.ll", 44, 1);
    assert!(request(&client, malformed.clone()).await.contents.is_err());
    let failing = lyric::source::request("(error \"failed\")", "failure.ll", 8, 1);
    assert!(request(&client, failing).await.contents.is_err());
    let Form::List(mut forms) = expected else {
        unreachable!()
    };
    forms.extend([compound, malformed, parse("(error \"failed\")").unwrap()]);
    assert_eq!(eval(&client, "(history)").await, Form::List(forms));
}

#[tokio::test]
async fn history_is_bounded_and_does_not_recursively_trace_evaluation() {
    let runtime = Runtime::new("test");
    let (client, _) = connect(&runtime).await;
    for n in 0..25 {
        assert_eq!(eval(&client, &format!("(+ {n} 1)")).await, Form::Int(n + 1));
    }
    let expected = Form::List(
        (5..25)
            .map(|n| parse(&format!("(+ {n} 1)")).unwrap())
            .collect(),
    );
    assert_eq!(eval(&client, "(history)").await, expected);
    for source in ["nil", "true", "42", "'(history)"] {
        eval(&client, source).await;
    }
    let Form::List(entries) = eval(&client, "(history)").await else {
        unreachable!()
    };
    assert_eq!(
        &entries[16..],
        &[
            Form::Nil,
            Form::Bool(true),
            Form::Int(42),
            parse("'(history)").unwrap()
        ]
    );
}

#[tokio::test]
async fn service_history_is_executable_and_quotes_data_arguments() {
    let runtime = Runtime::new("test");
    setup(
        &runtime,
        r#"
      (defn! echo (value) value)
      (defn! ping () :pong)
      (defn! fail () (error "failed"))
      (spawn_srv! :echo :interface '(echo ping fail))
    "#,
    )
    .await;
    let (client, _) = connect(&runtime).await;
    eval(&client, "(bind_srv :echo)").await;
    let item = parse("(:title \"Save\" :on_click (save_article (active_tab)))").unwrap();
    assert_eq!(
        eval(
            &client,
            "(echo '(:title \"Save\" :on_click (save_article (active_tab))))"
        )
        .await,
        item
    );
    assert_eq!(eval(&client, "(echo 'name)").await, Form::symbol("name"));
    assert_eq!(eval(&client, "(ping)").await, Form::keyword("pong"));
    assert!(request(&client, parse("(fail)").unwrap())
        .await
        .contents
        .is_err());
    assert_eq!(eval(&client, "(history :echo)").await, parse("((echo '(:title \"Save\" :on_click (save_article (active_tab)))) (echo 'name) (ping) (fail))").unwrap());
    // The first retained call is real code, not a raw selector message.
    assert_eq!(eval(&client, "(eval (get (history :echo) 0))").await, item);
}

#[tokio::test]
async fn capture_does_not_break_handlers_with_opaque_values_or_shadowing_parameters() {
    let runtime = Runtime::new("test");
    setup(
        &runtime,
        r#"
      (defn! execute (callback) (callback))
      (defn! named (history_append literal_form list map concat try named _)
        (+ history_append literal_form list map concat try named))
      (defn! same (list list) list)
      (spawn_srv! :worker :interface '(execute named same))
    "#,
    )
    .await;
    let (client, _) = connect(&runtime).await;
    eval(&client, "(bind_srv :worker)").await;
    assert_eq!(eval(&client, "(execute (fn () 42))").await, Form::Int(42));
    assert_eq!(
        eval(&client, "(history :worker)").await,
        parse("()").unwrap()
    );
    // Exercise the generated dispatcher directly. The existing bind_srv stub
    // itself uses `list`, which this deliberately hostile signature shadows.
    assert_eq!(
        eval(
            &client,
            "(call (find_srv :worker) '(:named 1 2 3 4 5 6 7 8))"
        )
        .await,
        Form::Int(28)
    );
    assert_eq!(
        eval(&client, "(history :worker)").await,
        parse("((named 1 2 3 4 5 6 7 8))").unwrap()
    );
    assert_eq!(
        eval(&client, "(call (find_srv :worker) '(:same 3 3))").await,
        Form::Int(3)
    );
    assert_eq!(
        eval(&client, "(call (find_srv :worker) '(:same 3 4))").await,
        parse("(:err \"Unrecognized message\")").unwrap()
    );
    assert_eq!(
        eval(&client, "(history :worker)").await,
        parse("((named 1 2 3 4 5 6 7 8) (same 3 3))").unwrap()
    );
}

#[tokio::test]
async fn service_replacement_has_new_history_and_old_pid_remains_readable() {
    let runtime = Runtime::new("test");
    setup(&runtime, COUNTER).await;
    let (client, _) = connect(&runtime).await;
    eval(&client, "(bind_srv :counter)").await;
    eval(&client, "(def old (find_srv :counter))").await;
    eval(&client, "(add_count 3)").await;
    setup(&runtime, COUNTER).await;
    assert_eq!(
        eval(&client, "(history :counter)").await,
        parse("()").unwrap()
    );
    assert_eq!(
        eval(&client, "(history old)").await,
        parse("((add_count 3))").unwrap()
    );
    eval(&client, "(add_count 4)").await;
    assert_eq!(
        eval(&client, "(history :counter)").await,
        parse("((add_count 4))").unwrap()
    );
    assert_eq!(
        eval(&client, "(history old)").await,
        parse("((add_count 3))").unwrap()
    );
}

#[tokio::test]
async fn history_can_be_read_while_service_waits_and_after_process_failure() {
    let runtime = Runtime::new("test");
    setup(
        &runtime,
        r#"
      (defn! hold () (publish :waiting :ready) (recv ':release))
      (spawn_srv! :worker :interface '(hold))
    "#,
    )
    .await;
    let (caller, _) = connect(&runtime).await;
    let (observer, _) = connect(&runtime).await;
    let mut ready = observer.subscribe("waiting".into()).await.unwrap();
    eval(&observer, ":subscription_barrier").await;
    eval(&caller, "(bind_srv :worker)").await;
    let (result, _) = tokio::join!(eval(&caller, "(hold)"), async {
        timeout(Duration::from_secs(3), ready.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            eval(&observer, "(history :worker)").await,
            parse("((hold))").unwrap()
        );
        eval(&observer, "(send (find_srv :worker) :release)").await;
    });
    assert_eq!(result, Form::keyword("release"));
    let failed = runtime
        .run(Program::from_script("(history_append '(work)) (error \"failed\")").unwrap())
        .await
        .unwrap();
    let pid = failed.id();
    assert!(failed.join().await.unwrap().status.is_err());
    assert_eq!(
        eval(&observer, &format!("(history (pid {}))", pid.inner())).await,
        parse("((work))").unwrap()
    );
}

#[tokio::test]
async fn pubsub_and_unrecognized_messages_do_not_become_function_history() {
    let runtime = Runtime::new("test");
    setup(
        &runtime,
        r#"
      (defn! changed (value) nil)
      (defn! ping () :pong)
      (spawn_srv! :events :interface '(ping) :topics '((:changed changed)))
    "#,
    )
    .await;
    let (client, _) = connect(&runtime).await;
    eval(
        &client,
        "(send (find_srv :events) '(:topic_updated :changed 42))",
    )
    .await;
    eval(&client, "(send (find_srv :events) :unrelated)").await;
    eval(&client, "(call (find_srv :events) '(:missing))").await;
    eval(&client, "(call (find_srv :events) '(:ping))").await;
    assert_eq!(
        eval(&client, "(history :events)").await,
        parse("((ping))").unwrap()
    );
}
