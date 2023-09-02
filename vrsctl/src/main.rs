use anyhow::{Context, Result};
use serde_json::json;
use std::io;
use tokio::net::UnixStream;
use vrs::connection::Connection;
use vrs::shell::Shell;

#[tokio::main]
async fn main() -> Result<()> {
    let path = vrs::runtime_socket()
        .with_context(|| "No path to runtime socket is configured".to_string())?;

    let conn = UnixStream::connect(&path)
        .await
        .with_context(|| format!("Failed to connect to socket {}", path.display()))?;

    println!("Connected to runtime: {:?}", conn);
    let conn = Connection::new(conn);
    let mut shell = Shell::new(conn);

    loop {
        let mut s = String::new();
        io::stdin()
            .read_line(&mut s)
            .expect("failed to read from stdin");
        let s = s.trim();
        if s == "exit" {
            break;
        }
        let resp = shell.request(json!({"message": s})).await;
        println!("Response: {:?}", resp);
    }

    Ok(())
}
