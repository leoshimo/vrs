use anyhow::{Context, Result};
use colored::*;

use std::fs::File;
use std::io::IsTerminal;
use std::io::{self, BufRead, BufReader, Read, Write};
use tokio::net::UnixStream;
use tracing::debug;
use vrs::client::Client;
use vrs::connection::Connection;

async fn run_cmd(mut client: Client, cmd: &str) -> Result<()> {
    let f = lemma::parse(cmd)?;
    let resp = client.request(f).await?;
    println!("{}", resp.contents);
    Ok(())
}

async fn run_repl(mut client: Client, read: impl Read, show_prompt: bool) -> Result<()> {
    let mut stream = BufReader::new(read);

    let mut s = String::new();
    while client.is_active() {
        if show_prompt && s.is_empty() {
            print!("{}", "vrs> ".bold().bright_white());
            io::stdout().flush()?;
        }

        match stream.read_line(&mut s) {
            Ok(0) | Err(_) => {
                client.shutdown().await;
                break;
            }
            _ => (),
        };

        if s.trim() == "exit" {
            client.shutdown().await;
            continue;
        }
        if s.is_empty() {
            continue;
        }
        if s.starts_with('#') {
            s.clear();
            continue;
        }

        let f = match lemma::parse(&s) {
            Ok(f) => {
                s.clear();
                f
            }
            Err(_) => continue, // parse failing is file - continue reading
        };

        let resp = match client.request(f).await {
            Ok(resp) => resp,
            Err(e) => {
                eprintln!("{}", e);
                continue;
            }
        };

        println!("{}", resp.contents);
    }
    Ok(())
}

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
    let client = Client::new(conn);

    // TODO: Build proper CLI
    let args = std::env::args().collect::<Vec<_>>();
    match args.get(1) {
        Some(cmd) => {
            let f = std::path::Path::new(cmd);
            if f.exists() {
                run_repl(client, File::open(f).unwrap(), false).await?;
            } else {
                run_cmd(client, cmd).await?;
            }
        }
        None => run_repl(client, io::stdin(), io::stdin().is_terminal()).await?,
    }

    Ok(())
}
