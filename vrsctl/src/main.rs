use anyhow::{Context, Result};
use std::io::{self, Write};
use tokio::net::UnixStream;
use tracing::debug;
use vrs::connection::Connection;
use vrs::shell::Shell;

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
    let mut shell = Shell::new(conn);

    loop {
        let mut s = String::new();
        print!("> ");
        io::stdout().flush()?;
        io::stdin()
            .read_line(&mut s)
            .expect("failed to read from stdin");
        let s = s.trim();
        if s == "exit" {
            break;
        }

        let f = lemma::parse(&s).with_context(|| format!("Invalid expression - {}", s))?;

        let resp = shell.request(f).await;
        match resp {
            Ok(resp) => println!("{}", resp.contents),
            Err(e) => println!("{}", e),
        }
    }

    Ok(())
}
