use anyhow::{Context, Result};
use tokio::net::UnixListener;

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
    }

    Ok(())
}
