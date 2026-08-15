use std::{process::Stdio, sync::Arc, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::UnixListener,
    process::Command,
    time::timeout,
};
use vrs::{Client, Connection, Form, Program, Runtime};

#[tokio::test]
async fn local_file_remote_execution_persistent_client_and_reload() {
    let alpha = Arc::new(Runtime::new("alpha"));
    let beta = Runtime::new("beta");
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    beta.listen_for_nodes(port).await.unwrap();
    beta.run(
        Program::from_expr("(begin (defn! ready () true) (spawn_srv! :ready :interface '(ready)))")
            .unwrap(),
    )
    .await
    .unwrap()
    .join()
    .await
    .unwrap();
    let (conn, daemon) = Connection::pair().unwrap();
    alpha.handle_conn(daemon).await.unwrap();
    let caller = Client::new(conn);
    caller
        .request(
            Form::from_expr(&format!("(configure :nodes '(\"tcp://127.0.0.1:{port}\"))")).unwrap(),
        )
        .await
        .unwrap()
        .contents
        .unwrap();
    caller
        .request(Form::from_expr("(wait_srv :ready :timeout 3)").unwrap())
        .await
        .unwrap()
        .contents
        .unwrap();
    let dir = std::env::temp_dir().join(format!("vrs-cli-remote-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let socket = dir.join("socket");
    let listener = UnixListener::bind(&socket).unwrap();
    let serving = tokio::spawn(async move {
        loop {
            let (conn, _) = listener.accept().await.unwrap();
            alpha.handle_conn(Connection::new(conn)).await.unwrap();
        }
    });
    let source = dir.join("local-only.ll");
    for revision in 1..=2 {
        std::fs::write(&source, format!("(defn! host_probe () (list (node_name) {revision}))\n(spawn_srv! :host_probe :interface '(host_probe))\n")).unwrap();
        let output = timeout(
            Duration::from_secs(5),
            Command::new(env!("CARGO_BIN_EXE_vrsctl"))
                .arg("--socket")
                .arg(&socket)
                .args(["--node", "beta"])
                .arg(&source)
                .output(),
        )
        .await
        .unwrap()
        .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let expression = if revision == 1 {
            "(begin (bind_srv :host_probe) (host_probe))"
        } else {
            "(host_probe)"
        };
        assert_eq!(
            caller
                .request(Form::from_expr(expression).unwrap())
                .await
                .unwrap()
                .contents
                .unwrap(),
            Form::List(vec![Form::string("beta"), Form::Int(revision)])
        );
    }
    let mut editor = Command::new(env!("CARGO_BIN_EXE_vrsctl"))
        .arg("--socket")
        .arg(&socket)
        .args(["--node", "beta", "--session"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let mut input = editor.stdin.take().unwrap();
    let mut lines = BufReader::new(editor.stdout.take().unwrap()).lines();
    for (source, expected) in [
        ("(def x 41)", "41\n"),
        ("(+ x 1)", "42\n"),
        ("(node_name)", "\"beta\"\n"),
    ] {
        input
            .write_all(
                format!(
                    "{}\n",
                    serde_json::json!({"source": source, "file": "/local/buffer.ll"})
                )
                .as_bytes(),
            )
            .await
            .unwrap();
        let line = timeout(Duration::from_secs(5), lines.next_line())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let reply: serde_json::Value = serde_json::from_str(&line).unwrap();
        assert_eq!(reply["ok"], true);
        assert_eq!(reply["output"], expected);
    }
    drop(input);
    assert!(timeout(Duration::from_secs(5), editor.wait())
        .await
        .unwrap()
        .unwrap()
        .success());
    serving.abort();
    std::fs::remove_dir_all(dir).unwrap();
}
