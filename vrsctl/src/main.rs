mod editor;

use anyhow::{Context, Result};
use clap::{arg, command};
use rustyline::error::ReadlineError;

use std::fs::File;
use std::io::{BufRead, BufReader};
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

    if let Some(cmd) = args.get_one::<String>("command") {
        run_cmd(client, cmd).await
    } else if let Some(file) = args.get_one::<String>("file") {
        run_file(client, file).await
    } else {
        run_repl(client).await
    }
}

/// The clap CLI interface
fn cli() -> clap::Command {
    command!()
        .arg(arg!(command: -c --command <COMMAND> "If present, COMMAND is sent and program exits"))
        .arg(arg!(file: [FILE] "If present, executes contents of FILE"))
}

/// Run a single request
async fn run_cmd(mut client: Client, cmd: &str) -> Result<()> {
    let f = lyric::parse(cmd)?;
    let resp = client.request(f).await?;
    match resp.contents {
        Ok(c) => println!("{}", c),
        Err(e) => eprintln!("{}", e),
    }
    Ok(())
}

async fn run_file(mut client: Client, file: &str) -> Result<()> {
    let f = File::open(file).with_context(|| format!("Failed to open {}", file))?;
    let mut f = BufReader::new(f);
    let mut line = String::new();
    loop {
        match f.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => (),
            Err(e) => {
                eprintln!("Error reading file - {}", e);
                break;
            }
        }

        if line.starts_with('#') {
            line.clear();
            continue;
        }

        let f = match lyric::parse(&line) {
            Ok(f) => f,
            Err(lyric::Error::IncompleteExpression(_)) => {
                continue;
            }
            Err(e) => {
                eprintln!("{} - {}", e, line);
                break;
            }
        };

        line.clear();

        match client.request(f).await {
            Ok(resp) => match resp.contents {
                Ok(c) => println!("{}", c),
                Err(e) => eprintln!("{}", e),
            },
            Err(e) => {
                eprintln!("{}", e);
            }
        }
    }

    Ok(())
}

/// Run an interactive REPL
async fn run_repl(mut client: Client) -> Result<()> {
    let mut rl = editor::editor()?;
    if let Some(history_file) = history_file() {
        if let Err(e) = rl.load_history(&history_file) {
            eprintln!("Failed to load {} - {}", history_file.to_string_lossy(), e);
        }
    }

    loop {
        match rl.readline("vrs> ") {
            Ok(line) => {
                if line.starts_with('#') {
                    continue; // skip shebang
                }

                let _ = rl.add_history_entry(line.as_str());

                let f = match lyric::parse(&line) {
                    Ok(f) => f,
                    Err(e) => {
                        eprintln!("{}", e);
                        continue;
                    }
                };
                match client.request(f).await {
                    Ok(resp) => match resp.contents {
                        Ok(c) => println!("{}", c),
                        Err(e) => eprintln!("{}", e),
                    },
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
