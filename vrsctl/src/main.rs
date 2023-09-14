use anyhow::{Context, Result};
use colored::*;
use lemma::Form;
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
        print!("{}", "vrs> ".bold().bright_white());
        io::stdout().flush()?;
        io::stdin()
            .read_line(&mut s)
            .expect("failed to read from stdin");
        let s = s.trim();
        if s == "exit" {
            client.shutdown().await;
            continue;
        }

        let f = match lemma::parse(s) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("{} {}", "ERROR:".red(), e);
                continue;
            }
        };

        let resp = match client.request(f).await {
            Ok(resp) => resp,
            Err(e) => {
                eprintln!("{}", e);
                continue;
            }
        };

        let resp_list = match resp.contents {
            Form::List(l) => l,
            form => {
                eprintln!("Unexpected response form - {}", form);
                continue;
            }
        };

        let (keyword, f) = match &resp_list[..] {
            [Form::Keyword(keyword), f] => (keyword, f),
            _ => {
                eprintln!("Unexpected response form");
                continue;
            }
        };

        println!(
            "{} {}",
            keyword.to_string().green(),
            f.to_string().bright_white()
        );
    }

    Ok(())
}
