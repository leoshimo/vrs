use anyhow::{Context, Result};
use serde_json::json;
use tokio::net::UnixStream;
use vrs::connection::Connection;
use vrs::message::Message;

#[tokio::main]
async fn main() -> Result<()> {
    let path = vrs::runtime_socket()
        .with_context(|| "No path to runtime socket is configured".to_string())?;

    let conn = UnixStream::connect(&path)
        .await
        .with_context(|| format!("Failed to connect to socket {}", path.display()))?;

    println!("Connected to runtime: {:?}", conn);
    let mut conn = Connection::new(conn);
    let msg = Message::new(json!({"message": "Hello"}));
    conn.send(&msg)
        .await
        .with_context(|| "Failed to send message".to_string())?;

    while let Some(msg) = conn.recv().await {
        match msg {
            Ok(msg) => println!(
                "Received response: {}",
                serde_json::to_string_pretty(&msg.0).unwrap()
            ),
            Err(e) => eprintln!("Received error: {}", e),
        }
    }

    Ok(())
}
