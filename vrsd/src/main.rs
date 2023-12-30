use anyhow::{Context, Result};
use tokio::net::UnixListener;
use tracing::{error, info};
use vrs::Connection;
use vrs::Runtime;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let path = vrs::runtime_socket();
    if path.exists() {
        std::fs::remove_file(&path)
            .with_context(|| format!("Failed to remove existing socket {}", path.display()))?;
    }

    let runtime = Runtime::new();

    let listener = UnixListener::bind(&path)
        .with_context(|| format!("Failed to start listener at {}", path.display()))?;

    loop {
        match listener.accept().await {
            Ok((conn, _addr)) => {
                info!("Connected to client: {:?}", conn);
                let conn = Connection::new(conn);
                runtime.handle_conn(conn).await?;
            }
            Err(e) => {
                error!("Unable to accept connections - {e}");
            }
        }
    }
}
