use anyhow::{Context, Result};
use colored::*;
use std::io::{self, Write};
use tokio::net::UnixStream;
use tracing::debug;
use vrs::client::Client;
use vrs::connection::Connection;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let path = vrs::runtime_socket()
        .with_context(|| "No path to runtime socket is configured".to_string())?;

    let conn = UnixStream::connect(&path)
        .await
        .with_context(|| format!("Failed to connect to socket {}", path.display()))?;

    debug!("Connected to runtime: {:?}", conn);
    let conn = Connection::new(conn);
    let mut client = Client::new(conn);

    while client.is_active() {
        let mut s = String::new();
        print!("> ");
        io::stdout().flush()?;
        io::stdin()
            .read_line(&mut s)
            .expect("failed to read from stdin");
        let s = s.trim();
        if s == "exit" {
            client.shutdown().await;
            continue;
        }

        let f = lemma::parse(s).with_context(|| format!("Invalid expression - {}", s))?;
        let resp = client.request(f).await;
        match resp {
            Ok(resp) => println!("{}", resp.contents.to_string().green()),
            Err(e) => eprintln!("{}", e),
        }
    }

    Ok(())
}
