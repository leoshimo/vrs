use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use tokio::net::UnixStream;
use tokio::process::{Child, Command};
use tokio::time::{sleep, timeout, Instant};
use vrs::{Client, Connection, Form};

struct TestDir(PathBuf);

static TEST_DIR_SEQUENCE: AtomicU64 = AtomicU64::new(0);

impl TestDir {
    fn new() -> Self {
        loop {
            let sequence = TEST_DIR_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path =
                std::env::temp_dir().join(format!("vpt-{:x}-{sequence:x}", std::process::id()));
            match std::fs::create_dir(&path) {
                Ok(()) => return Self(path),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("failed to create test directory: {error}"),
            }
        }
    }

    fn join(&self, path: &str) -> PathBuf {
        self.0.join(path)
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct Daemon(Child);

impl Drop for Daemon {
    fn drop(&mut self) {
        let _ = self.0.start_kill();
    }
}

fn node_ports() -> (u16, u16) {
    let alpha = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let beta = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let ports = (
        alpha.local_addr().unwrap().port(),
        beta.local_addr().unwrap().port(),
    );
    drop((alpha, beta));
    ports
}

fn spawn_vrsd(node: &str, port: u16, socket: &Path, init: &Path) -> Daemon {
    let mut command = Command::new(env!("CARGO_BIN_EXE_vrsd"));
    command
        .arg("--node")
        .arg(node)
        .arg("--node-port")
        .arg(port.to_string())
        .arg("--socket")
        .arg(socket)
        .arg("--init")
        .arg(init)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    Daemon(command.spawn().unwrap())
}

async fn connect_client(socket: &Path) -> Client {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match UnixStream::connect(socket).await {
            Ok(stream) => return Client::new(Connection::new(stream)),
            Err(error) if Instant::now() < deadline => {
                let _ = error;
                sleep(Duration::from_millis(20)).await;
            }
            Err(error) => panic!("failed to connect to {}: {error}", socket.display()),
        }
    }
}

async fn service_is_visible(client: &Client) -> bool {
    client
        .request(Form::from_expr("(find_srv :remote_probe)").unwrap())
        .await
        .unwrap()
        .contents
        .is_ok()
}

async fn service_count(client: &Client, name: &str) -> usize {
    let response = client
        .request(Form::from_expr("(ls_srv)").unwrap())
        .await
        .unwrap()
        .contents
        .unwrap();
    let Form::List(entries) = response else {
        panic!("ls_srv should return a list");
    };
    entries
        .chunks_exact(2)
        .filter(|entry| entry[0] == Form::keyword(name))
        .count()
}

async fn wait_for_value(client: &Client, expression: &str, expected: &Form, failure: &str) {
    let request = Form::from_expr(expression).unwrap();
    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        let response = client.request(request.clone()).await.unwrap().contents;
        if response.as_ref() == Ok(expected) {
            return;
        }
        assert!(Instant::now() < deadline, "{failure}: {response:?}");
        sleep(Duration::from_millis(50)).await;
    }
}

#[tokio::test]
async fn configured_node_reconnects_and_routes_service_calls() {
    let test_dir = TestDir::new();
    let alpha_socket = test_dir.join("alpha.socket");
    let beta_socket = test_dir.join("beta.socket");
    let alpha_init = test_dir.join("alpha.ll");
    let beta_init = test_dir.join("beta.ll");
    let (alpha_port, beta_port) = node_ports();

    std::fs::write(
        &alpha_init,
        format!("(configure :nodes '(\"tcp://127.0.0.1:{beta_port}\"))"),
    )
    .unwrap();
    std::fs::write(
        &beta_init,
        concat!(
            "(defn ping (message) (list :pong message))\n",
            "(spawn_srv :remote_probe :interface '(ping))",
        ),
    )
    .unwrap();

    let _alpha = spawn_vrsd("alpha", alpha_port, &alpha_socket, &alpha_init);
    let client = connect_client(&alpha_socket).await;

    let initial_lookup = timeout(
        Duration::from_millis(500),
        client.request(Form::from_expr("(find_srv :remote_probe)").unwrap()),
    )
    .await
    .expect("find_srv should only consult the cache")
    .unwrap();
    assert!(initial_lookup.contents.is_err());

    let beta = spawn_vrsd("beta", beta_port, &beta_socket, &beta_init);
    let beta_client = connect_client(&beta_socket).await;

    let deadline = Instant::now() + Duration::from_secs(8);
    while !service_is_visible(&client).await {
        assert!(
            Instant::now() < deadline,
            "alpha never received beta's service registry"
        );
        sleep(Duration::from_millis(50)).await;
    }

    let response = client
        .request(Form::from_expr("(begin (bind_srv :remote_probe) (ping \"hello\"))").unwrap())
        .await
        .unwrap()
        .contents
        .unwrap();
    assert_eq!(
        response,
        Form::List(vec![Form::keyword("pong"), Form::string("hello")])
    );

    drop(beta_client);
    drop(beta);
    let deadline = Instant::now() + Duration::from_secs(8);
    while service_is_visible(&client).await {
        assert!(
            Instant::now() < deadline,
            "alpha retained beta's services after beta disconnected"
        );
        sleep(Duration::from_millis(50)).await;
    }

    let _restarted_beta = spawn_vrsd("beta", beta_port, &beta_socket, &beta_init);
    let _restarted_beta_client = connect_client(&beta_socket).await;
    let deadline = Instant::now() + Duration::from_secs(8);
    while !service_is_visible(&client).await {
        assert!(
            Instant::now() < deadline,
            "alpha never reconnected to beta's service registry"
        );
        sleep(Duration::from_millis(50)).await;
    }

    let response = client
        .request(Form::from_expr("(begin (bind_srv :remote_probe) (ping \"again\"))").unwrap())
        .await
        .unwrap()
        .contents
        .unwrap();
    assert_eq!(
        response,
        Form::List(vec![Form::keyword("pong"), Form::string("again")])
    );
}

#[tokio::test]
async fn republished_service_replaces_previous_remote_registration() {
    let test_dir = TestDir::new();
    let alpha_socket = test_dir.join("alpha.socket");
    let beta_socket = test_dir.join("beta.socket");
    let alpha_init = test_dir.join("alpha.ll");
    let beta_init = test_dir.join("beta.ll");
    let (alpha_port, beta_port) = node_ports();

    std::fs::write(
        &alpha_init,
        format!("(configure :nodes '(\"tcp://127.0.0.1:{beta_port}\"))"),
    )
    .unwrap();
    std::fs::write(&beta_init, ":ok").unwrap();

    let _alpha = spawn_vrsd("alpha", alpha_port, &alpha_socket, &alpha_init);
    let alpha_client = connect_client(&alpha_socket).await;
    let _beta = spawn_vrsd("beta", beta_port, &beta_socket, &beta_init);
    let beta_client = connect_client(&beta_socket).await;

    beta_client
        .request(
            Form::from_expr(
                "(begin
                    (defn first_hook (cmd) cmd)
                    (spawn_srv :replaceable :interface '(first_hook)))",
            )
            .unwrap(),
        )
        .await
        .unwrap()
        .contents
        .unwrap();

    let deadline = Instant::now() + Duration::from_secs(8);
    while service_count(&alpha_client, "replaceable").await == 0 {
        assert!(
            Instant::now() < deadline,
            "alpha never received beta's first registration"
        );
        sleep(Duration::from_millis(50)).await;
    }

    beta_client
        .request(
            Form::from_expr(
                "(begin
                    (defn second_hook (expr) (list :second expr))
                    (spawn_srv :replaceable :interface '(second_hook)))",
            )
            .unwrap(),
        )
        .await
        .unwrap()
        .contents
        .unwrap();

    let deadline = Instant::now() + Duration::from_secs(8);
    let response = loop {
        let response = alpha_client
            .request(
                Form::from_expr(
                    "(begin
                        (bind_srv :replaceable)
                        (second_hook \"new\"))",
                )
                .unwrap(),
            )
            .await
            .unwrap()
            .contents;
        let expected = Form::List(vec![Form::keyword("second"), Form::string("new")]);
        if response.as_ref() == Ok(&expected) {
            break response.unwrap();
        }
        assert!(
            Instant::now() < deadline,
            "alpha never received beta's replacement registration"
        );
        sleep(Duration::from_millis(50)).await;
    };

    assert_eq!(
        response,
        Form::List(vec![Form::keyword("second"), Form::string("new")])
    );
    assert_eq!(service_count(&alpha_client, "replaceable").await, 1);
}

#[tokio::test]
async fn later_registration_shadows_same_named_service_on_another_node() {
    let test_dir = TestDir::new();
    let alpha_socket = test_dir.join("alpha.socket");
    let beta_socket = test_dir.join("beta.socket");
    let alpha_init = test_dir.join("alpha.ll");
    let beta_init = test_dir.join("beta.ll");
    let (alpha_port, beta_port) = node_ports();

    std::fs::write(
        &alpha_init,
        format!("(configure :nodes '(\"tcp://127.0.0.1:{beta_port}\"))"),
    )
    .unwrap();
    std::fs::write(&beta_init, ":ok").unwrap();

    let _alpha = spawn_vrsd("alpha", alpha_port, &alpha_socket, &alpha_init);
    let alpha_client = connect_client(&alpha_socket).await;
    let _beta = spawn_vrsd("beta", beta_port, &beta_socket, &beta_init);
    let beta_client = connect_client(&beta_socket).await;

    alpha_client
        .request(
            Form::from_expr(
                "(begin
                    (defn first_local_hook () :first_local)
                    (spawn_srv :shared :interface '(first_local_hook)))",
            )
            .unwrap(),
        )
        .await
        .unwrap()
        .contents
        .unwrap();
    wait_for_value(
        &alpha_client,
        "(begin (bind_srv :shared) (first_local_hook))",
        &Form::keyword("first_local"),
        "alpha never selected its initial local service",
    )
    .await;
    assert_eq!(service_count(&alpha_client, "shared").await, 1);

    beta_client
        .request(
            Form::from_expr(
                "(begin
                    (defn remote_hook () :remote)
                    (spawn_srv :shared :interface '(remote_hook)))",
            )
            .unwrap(),
        )
        .await
        .unwrap()
        .contents
        .unwrap();
    wait_for_value(
        &alpha_client,
        "(begin (bind_srv :shared) (remote_hook))",
        &Form::keyword("remote"),
        "beta's later service never shadowed alpha's local service",
    )
    .await;
    assert_eq!(service_count(&alpha_client, "shared").await, 1);

    alpha_client
        .request(
            Form::from_expr(
                "(begin
                    (defn second_local_hook () :second_local)
                    (spawn_srv :shared :interface '(second_local_hook)))",
            )
            .unwrap(),
        )
        .await
        .unwrap()
        .contents
        .unwrap();
    wait_for_value(
        &alpha_client,
        "(begin (bind_srv :shared) (second_local_hook))",
        &Form::keyword("second_local"),
        "alpha's later local service never shadowed beta's remote service",
    )
    .await;
    assert_eq!(service_count(&alpha_client, "shared").await, 1);
}
