use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tokio::net::UnixStream;
use tokio::process::{Child, Command};
use tokio::time::{sleep, timeout, Instant};
use vrs::{Client, Connection, Form};

struct TestDir(PathBuf);

impl TestDir {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "vrsd-peer-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
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
