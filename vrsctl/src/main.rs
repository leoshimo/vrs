mod editor;
mod repl;
mod watch;

use anyhow::{Context, Result};
use clap::{arg, command, ArgGroup};

use std::fs::File;
use std::io::{self, BufRead, BufReader, IsTerminal, Read};
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
        let file = open_file(
            args.get_one::<String>("file")
                .expect("file has a default value"),
        )?;

        if let Some(cmd) = args.get_one::<String>("command") {
            run_cmd(&client, cmd).await
        } else if let Some(file) = file {
            run_file(&client, file).await
        } else if let Some(topic) = args.get_one::<String>("subscribe") {
            let follow = args.get_flag("follow");
            let follow_clear = args.get_flag("follow_clear");
            watch::run(
                &client,
                KeywordId::from(topic.as_str()),
                watch::Opts {
                    follow: follow || follow_clear,
                    clear: follow_clear,
                },
            )
            .await
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
        .arg(arg!(file: [FILE] "If present, executes contents of FILE")
             .default_value("-"))
        .arg(arg!(command: -c --command <EXPR> "If present, EXPR is sent as request, then program exits"))
        .arg(arg!(subscribe: -s --subscribe <TOPIC> "If present, watches a specific topic for data"))
        .group(ArgGroup::new("main")
               .args(["command", "subscribe"])
               .required(false))
        .arg(arg!(follow: -f --follow "If present, continues polling subscription after first topic update")
             .requires("subscribe"))
        .arg(arg!(follow_clear: -F --followclear "Like --follow, but clears screen after each value")
            .requires("subscribe"))
        .arg(
            arg!(socket: -S --socket <SOCKET> "Path to unix socket for vrsd")
                .default_value(vrs::runtime_socket().into_os_string()),
        )
}

/// Open file specified by argument
fn open_file(file: &str) -> Result<Option<Box<dyn Read>>> {
    match file {
        "-" => {
            let stdin = io::stdin();
            if stdin.is_terminal() {
                Ok(None) // ignore "-" if interactive
            } else {
                Ok(Some(Box::new(stdin)))
            }
        }
        _ => Ok(Some(Box::new(File::open(file)?))),
    }
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
async fn run_file(client: &Client, file: Box<dyn Read>) -> Result<()> {
    let mut f = BufReader::new(file);
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

// TODO: Test case for executing from stdin
// TODO: Test case for executing from REPL
// TODO: Test case for executing from -c CMD
