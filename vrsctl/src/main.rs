use anyhow::{Context, Result};
use clap::{arg, command};
use rustyline::error::ReadlineError;

use std::path::PathBuf;
use tokio::net::UnixStream;
use tracing::debug;
use vrs::client::Client;
use vrs::connection::Connection;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let args = cli().get_matches();

    let path = vrs::runtime_socket()
        .with_context(|| "No path to runtime socket is configured".to_string())?;
    let conn = UnixStream::connect(&path)
        .await
        .with_context(|| format!("Failed to connect to socket {}", path.display()))?;

    debug!("Connected to runtime: {:?}", conn);
    let conn = Connection::new(conn);
    let client = Client::new(conn);

    match args.get_one::<String>("command") {
        Some(cmd) => run_cmd(client, cmd).await,
        None => run_repl(client).await,
    }
}

/// The clap CLI interface
fn cli() -> clap::Command {
    command!()
        .arg(arg!(command: -c --command <COMMAND> "If present, COMMAND is sent and program exits"))
}

/// Run a single request
async fn run_cmd(mut client: Client, cmd: &str) -> Result<()> {
    let f = lemma::parse(cmd)?;
    let resp = client.request(f).await?;
    println!("{}", resp.contents);
    Ok(())
}

/// Run an interactive REPL
async fn run_repl(mut client: Client) -> Result<()> {
    let mut rl = rustyline::DefaultEditor::new().with_context(|| "Failed to create line editor")?;
    if let Some(history_file) = history_file() {
        if let Err(e) = rl.load_history(&history_file) {
            eprintln!("Failed to load {} - {}", history_file.to_string_lossy(), e);
        }
    }

    loop {
        match rl.readline("vrs> ") {
            Ok(line) => {
                if line.starts_with("#!") {
                    continue; // skip shebang
                }

                let _ = rl.add_history_entry(line.as_str());

                let f = lemma::parse(&line).with_context(|| "Failed to parse line".to_string())?;
                match client.request(f).await {
                    Ok(resp) => {
                        println!("{}", resp.contents);
                    }
                    Err(e) => {
                        eprintln!("{}", e);
                    }
                }
            }
            Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => break,
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }

    client.shutdown().await;

    if let Some(history_file) = history_file() {
        if let Err(e) = rl.save_history(&history_file) {
            eprintln!("Failed to save {} - {}", history_file.to_string_lossy(), e);
        }
    }

    Ok(())
}

/// Path to file to use for history
fn history_file() -> Option<PathBuf> {
    let dir = dirs::data_local_dir()
        .or_else(dirs::data_dir)
        .or_else(dirs::home_dir)?;
    Some(dir.as_path().join(".vrsctl_history"))
}
