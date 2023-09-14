use anyhow::{Context, Result};
use colored::*;
use lemma::{Form, KeywordId};
use std::io::{self, Write};
use tokio::net::UnixStream;
use tracing::debug;
use vrs::client::Client;
use vrs::connection::Connection;
use vrs::Response;

async fn run_cmd(mut client: Client, cmd: &str) -> Result<()> {
    let f = lemma::parse(cmd)?;
    let resp = client.request(f).await?;
    match parse_resp(resp) {
        Some((_keyword, f)) => {
            println!("{}", f);
        }
        None => {
            eprintln!("Unexpected response format");
        }
    };

    Ok(())
}

async fn run_repl(mut client: Client) -> Result<()> {
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

        let (keyword, f) = match parse_resp(resp) {
            Some(val) => val,
            None => {
                eprintln!("Unexpected response format");
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

/// Compensate for some TBD plumbing
fn parse_resp(resp: Response) -> Option<(KeywordId, Form)> {
    let resp_list = match resp.contents {
        Form::List(l) => l,
        form => {
            eprintln!("Unexpected response form - {}", form);
            return None;
        }
    };

    match &resp_list[..] {
        [Form::Keyword(keyword), f] => Some((keyword.clone(), f.clone())),
        _ => None,
    }
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
        Some(cmd) => run_cmd(client, cmd).await?,
        None => run_repl(client).await?,
    }

    Ok(())
}
