use anyhow::{Context, Result};
use serde_json::json;
use tokio::net::UnixListener;
use tracing::{debug, error};
use vrs::connection::{Connection, Message};
use vrs::message::Response;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let path = vrs::runtime_socket()
        .with_context(|| "No path to runtime socket is configured".to_string())?;
    if path.exists() {
        std::fs::remove_file(&path)
            .with_context(|| format!("Failed to remove existing socket {}", path.display()))?;
    }

    let listener = UnixListener::bind(&path)
        .with_context(|| format!("Failed to start listener at {}", path.display()))?;

    // TODO: Replace with runtime event loop
    while let Ok((conn, _addr)) = listener.accept().await {
        debug!("Connected to client: {:?}", conn);
        tokio::spawn(async move {
            let mut conn = Connection::new(conn);
            while let Some(msg) = conn.recv().await {
                match msg {
                    Ok(msg) => {
                        if let Message::Request(req) = msg {
                            let resp = Message::Response(Response {
                                req_id: req.req_id,
                                contents: json!({"message": format!("GOT: {}", req.contents["message"])}),
                            });
                            conn.send(&resp).await.unwrap();
                        }
                    }
                    Err(e) => error!("Received error: {}", e),
                }
            }
        });
    }

    Ok(())
}
