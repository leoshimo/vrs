use anyhow::{Context, Result};
use tokio::net::UnixListener;
use tracing::debug;
use vrs::connection::Connection;
use vrs::Runtime;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let path = vrs::runtime_socket()
        .with_context(|| "No path to runtime socket is configured".to_string())?;
    if path.exists() {
        std::fs::remove_file(&path)
            .with_context(|| format!("Failed to remove existing socket {}", path.display()))?;
    }

    let runtime = Runtime::new();

    let listener = UnixListener::bind(&path)
        .with_context(|| format!("Failed to start listener at {}", path.display()))?;

    while let Ok((conn, _addr)) = listener.accept().await {
        debug!("Connected to client: {:?}", conn);
        let conn = Connection::new(conn);
        runtime.handle_conn(conn).await?;
    }

    Ok(())
}
