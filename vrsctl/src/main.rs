mod editor;
mod repl;
mod watch;

use anyhow::{Context, Result};
use clap::{arg, command};

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::str::FromStr;
use tokio::net::UnixStream;
use tracing::debug;
use vrs::{Client, Connection, KeywordId};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let args = cli().get_matches();

    let path = args
        .get_one::<String>("socket")
        .map(|s| PathBuf::from_str(s))
        .with_context(|| "No path to runtime socket is configured".to_string())??;

    let conn = UnixStream::connect(&path)
        .await
        .with_context(|| format!("Failed to connect to socket {}", path.display()))?;

    debug!("Connected to runtime: {:?}", conn);
    let conn = Connection::new(conn);
    let client = Client::new(conn);

    let run = async {
        if let Some(cmd) = args.get_one::<String>("command") {
            run_cmd(&client, cmd).await
        } else if let Some(file) = args.get_one::<String>("file") {
            run_file(&client, file).await
        } else if let Some(topic) = args.get_one::<String>("subscription") {
            let follow = args.get_flag("follow");
            watch::run(&client, KeywordId::from(topic.as_str()), follow).await
        } else {
            repl::run(&client).await
        }
    };

    tokio::select! {
        biased;
        res = run => {
            if let Err(e) = res {
                eprintln!("Terminated with error: {e}");
            }
        },
        _ = client.closed() => {
            eprintln!("Connection closed");
        }
    }

    Ok(())
}

/// The clap CLI interface
fn cli() -> clap::Command {
    command!()
        .arg(arg!(command: -c --command <COMMAND> "If present, COMMAND is sent and program exits"))
        .arg(arg!(file: [FILE] "If present, executes contents of FILE"))
        .arg(arg!(subscription: -s --subscription <TOPIC> "If present, watches a specific topic for data"))
        .arg(arg!(follow: -f --follow "If present, continues polling subscription after first topic update")
             .requires("subscription"))
        .arg(
            arg!(socket: -S --socket <SOCKET> "Path to unix socket for vrsd")
                .default_value(vrs::runtime_socket().into_os_string()),
        )
}

/// Run a single request
async fn run_cmd(client: &Client, cmd: &str) -> Result<()> {
    let f = lyric::parse(cmd)?;
    let resp = client.request(f).await?;
    match resp.contents {
        Ok(c) => println!("{}", c),
        Err(e) => eprintln!("{}", e),
    }
    Ok(())
}

/// Run a script file
async fn run_file(client: &Client, file: &str) -> Result<()> {
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
