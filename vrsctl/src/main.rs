use anyhow::{Context, Result};
use tokio::net::UnixStream;

#[tokio::main]
async fn main() -> Result<()> {
    let path = vrs::runtime_socket()
        .with_context(|| "No path to runtime socket is configured".to_string())?;

    let conn = UnixStream::connect(&path)
        .await
        .with_context(|| format!("Failed to connect to socket {}", path.display()))?;
    println!("Connected to runtime: {:?}", conn);

    Ok(())
}
