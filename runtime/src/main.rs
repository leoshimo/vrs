use anyhow::{Context, Result};
use serde_json::json;
use tokio::net::UnixListener;
use vrs::connection::{Connection, Message};

#[tokio::main]
async fn main() -> Result<()> {
    let path = vrs::runtime_socket()
        .with_context(|| "No path to runtime socket is configured".to_string())?;
    if path.exists() {
        std::fs::remove_file(&path)
            .with_context(|| format!("Failed to remove existing socket {}", path.display()))?;
    }

    let listener = UnixListener::bind(&path)
        .with_context(|| format!("Failed to start listener at {}", path.display()))?;

    while let Ok((conn, _addr)) = listener.accept().await {
        println!("Connected to client: {:?}", conn);
        tokio::spawn(async move {
            let mut conn = Connection::new(conn);
            while let Some(msg) = conn.recv().await {
                match msg {
                    Ok(msg) => {
                        println!(
                            "Received message: {}",
                            serde_json::to_string_pretty(&msg).unwrap()
                        );

                        if let Message::Request { request_id, .. } = msg {
                            let resp = Message::Response {
                                request_id,
                                contents: json!({"message": "Goodbye"}),
                            };
                            conn.send(&resp).await.unwrap();
                        }
                    }
                    Err(e) => eprintln!("Received error: {}", e),
                }
            }
        });
    }

    Ok(())
}
