use std::time::Duration;
use tokio::time::timeout;
use vrs::{Client, Connection, Form, Program, Runtime};

async fn client(runtime: &Runtime) -> Client {
    let (client, daemon) = Connection::pair().unwrap();
    runtime.handle_conn(daemon).await.unwrap();
    Client::new(client)
}

async fn eval(client: &Client, source: &str) -> Form {
    timeout(
        Duration::from_secs(5),
        client.request(Form::from_expr(source).unwrap()),
    )
    .await
    .expect("request timed out")
    .unwrap()
    .contents
    .unwrap()
}

async fn nodes() -> (Runtime, Runtime, Client, Client) {
    let alpha = Runtime::new("alpha");
    let beta = Runtime::new("beta");
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    beta.listen_for_nodes(port).await.unwrap();
    beta.run(
        Program::from_expr(
            "(begin (defn! link_probe () :ok) (spawn_srv! :link_probe :interface '(link_probe)))",
        )
        .unwrap(),
    )
    .await
    .unwrap()
    .join()
    .await
    .unwrap();
    let a = client(&alpha).await;
    let b = client(&beta).await;
    eval(
        &a,
        &format!("(configure :nodes '(\"tcp://127.0.0.1:{port}\"))"),
    )
    .await;
    eval(&a, "(wait_srv :link_probe :timeout 3)").await;
    (alpha, beta, a, b)
}

#[tokio::test]
async fn remote_code_is_quoted_isolated_and_returns_real_pids() {
    let (_alpha, _beta, a, _) = nodes().await;
    assert_eq!(
        eval(&a, "(remote! \"beta\" (node_name))").await,
        Form::string("beta")
    );
    assert_eq!(
        eval(&a, "(begin (def n 9) (eval_remote \"beta\" `(+ ,n 1)))").await,
        Form::Int(10)
    );
    assert_eq!(
        eval(&a, "(remote! \"beta\" (def x 42) x)").await,
        Form::Int(42)
    );
    assert_eq!(
        eval(&a, "(err? (try (remote! \"beta\" x)))").await,
        Form::Bool(true)
    );
    assert_eq!(
        eval(&a, "(err? (try (remote! \"missing\" 42)))").await,
        Form::Bool(true)
    );
    assert_eq!(
        eval(&a, "(err? (try (remote! \"beta\" (fn () 42))))").await,
        Form::Bool(true)
    );
    assert_eq!(
        eval(&a, "(remote! \"alpha\" (node_name))").await,
        Form::string("alpha")
    );
    assert_eq!(eval(&a, "(begin (def target_count 0) (remote! (begin (set target_count (+ target_count 1)) \"beta\") 42) target_count)").await, Form::Int(1));
}

#[tokio::test]
async fn forked_service_is_immediately_bindable_and_existing_stub_follows_reload() {
    let (alpha, _beta, a, _) = nodes().await;
    let caller = client(&alpha).await;
    eval(&a, "(def deployed (remote! \"beta\" (defn! probe () (list (node_name) 1)) (spawn_srv! :probe :interface '(probe))))").await;
    // No sleep or registry polling between deploy completion and another client binding.
    assert_eq!(
        eval(&caller, "(begin (bind_srv :probe) (probe))").await,
        Form::List(vec![Form::string("beta"), Form::Int(1)])
    );
    assert_eq!(
        eval(
            &a,
            "(eq? deployed (wait_srv :probe :pid deployed :timeout 0))"
        )
        .await,
        Form::Bool(true)
    );
    assert_eq!(
        eval(&a, "(call deployed '(:probe))").await,
        Form::List(vec![Form::string("beta"), Form::Int(1)])
    );
    for revision in 2..12 {
        eval(&a, &format!("(remote! \"beta\" (defn! probe () (list (node_name) {revision})) (spawn_srv! :probe :interface '(probe)))")).await;
        assert_eq!(
            eval(&caller, "(probe)").await,
            Form::List(vec![Form::string("beta"), Form::Int(revision)])
        );
    }
    assert_eq!(
        eval(
            &a,
            "(err? (try (wait_srv :probe :pid deployed :timeout 0)))"
        )
        .await,
        Form::Bool(true)
    );
}

#[tokio::test]
async fn remote_session_preserves_source_definitions_macros_errors_and_subscriptions() {
    let (alpha, _beta, _a, _) = nodes().await;
    let remote = client(&alpha).await;
    remote.select_node("beta").await.unwrap().contents.unwrap();
    eval(&remote, "(def x 20)").await;
    eval(&remote, "(defmacro twice (x) `(+ ,x ,x))").await;
    assert!(remote
        .request(Form::from_expr("(error \"expected\")").unwrap())
        .await
        .unwrap()
        .contents
        .is_err());
    assert_eq!(eval(&remote, "(twice! x)").await, Form::Int(40));
    let result = remote
        .request(lyric::source::request(
            "(node_name)",
            "/local/only/probe.ll",
            10,
            2,
        ))
        .await
        .unwrap();
    assert_eq!(result.contents.unwrap(), Form::string("beta"));
    let mut subscription = remote.subscribe("remote_topic".into()).await.unwrap();
    eval(&remote, "(publish :remote_topic 42)").await;
    assert_eq!(
        timeout(Duration::from_secs(2), subscription.recv())
            .await
            .unwrap()
            .unwrap(),
        Form::Int(42)
    );
    // A CLI session reply also waits for registration sync at the initiating node.
    eval(&remote, "(begin (defn! remote_session_probe () :ready) (spawn_srv! :remote_session_probe :interface '(remote_session_probe)))").await;
    let caller = client(&alpha).await;
    assert_eq!(
        eval(
            &caller,
            "(begin (bind_srv :remote_session_probe) (remote_session_probe))"
        )
        .await,
        Form::keyword("ready")
    );
}

#[tokio::test]
async fn read_script_sends_callers_source_without_requiring_remote_file() {
    let (_alpha, _beta, a, _) = nodes().await;
    let path = std::env::temp_dir().join(format!("vrs-remote-source-{}.ll", std::process::id()));
    std::fs::write(&path, "(def x 40)\n(+ x 2)\n").unwrap();
    assert_eq!(
        eval(
            &a,
            &format!(
                "(eval_remote \"beta\" (read_script {}))",
                Form::string(path.to_str().unwrap())
            )
        )
        .await,
        Form::Int(42)
    );
    std::fs::remove_file(path).unwrap();
}

#[tokio::test]
async fn pid_wait_ignores_old_registration_and_wakes_when_replacement_arrives() {
    let runtime = Runtime::new("waiter");
    let owner = client(&runtime).await;
    eval(
        &owner,
        "(begin (defn! probe () 1) (spawn_srv! :replace_me :interface '(probe)))",
    )
    .await;
    eval(
        &owner,
        "(def future_pid (spawn (fn () (recv :start) (srv! :replace_me :interface '(probe)))))",
    )
    .await;
    // Waiting in a child keeps the test independent of terminal request serialization.
    eval(&owner, "(def parent (self))").await;
    eval(
        &owner,
        "(spawn (fn () (send parent (list :visible (wait_srv :replace_me :pid future_pid)))))",
    )
    .await;
    eval(&owner, "(send future_pid :start)").await;
    assert_eq!(
        eval(&owner, "(eq? future_pid (get (recv '(:visible _)) 1))").await,
        Form::Bool(true)
    );
}

#[tokio::test]
async fn cancelling_remote_evaluation_and_closing_session_release_remote_processes() {
    let (alpha, _beta, a, b) = nodes().await;
    // Evaluator registers a marker before blocking. Killing its local owner
    // cancels the pending RPC and must kill the remote evaluator as well.
    eval(
        &a,
        "(def worker (spawn (fn () (remote! \"beta\" (register :blocked_evaluator) (recv)))))",
    )
    .await;
    eval(&a, "(wait_srv :blocked_evaluator :timeout 3)").await;
    eval(&a, "(kill worker)").await;
    timeout(Duration::from_secs(3), async {
        while eval(&b, "(ok? (try (find_srv :blocked_evaluator)))").await == Form::Bool(true) {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let session = client(&alpha).await;
    session.select_node("beta").await.unwrap().contents.unwrap();
    eval(&session, "(register :remote_editor)").await;
    session.shutdown().await;
    timeout(Duration::from_secs(3), async {
        while eval(&b, "(ok? (try (find_srv :remote_editor)))").await == Form::Bool(true) {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}
