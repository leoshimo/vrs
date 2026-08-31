mod editor;
mod repl;
mod watch;

use anyhow::{Context, Result};
use clap::builder::EnumValueParser;
use clap::{arg, command, ArgAction, ArgGroup};

use std::fs::File;
use std::io::{self, BufRead, BufReader, IsTerminal, Read};
use std::path::PathBuf;
use std::str::FromStr;
use tokio::net::UnixStream;
use tracing::debug;
use vrs::{Client, Connection, Form, KeywordId};

#[derive(clap::ValueEnum, Debug, Clone, PartialEq)]
enum Format {
    #[clap(help = "Default output format")]
    Default,
    #[clap(help = "Format for editors")]
    Editor,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let args = cli().get_matches();

    let path = args
        .get_one::<String>("socket")
        .map(|s| PathBuf::from_str(s))
        .with_context(|| "No path to runtime socket is configured")??;

    let conn = UnixStream::connect(&path)
        .await
        .with_context(|| format!("Failed to connect to socket {}", path.display()))?;

    debug!("Connected to runtime: {:?}", conn);
    let conn = Connection::new(conn);
    let client = Client::new(conn);

    let run = async {
        if let Some(name) = args.get_one::<String>("name") {
            let reg_req = Form::from_expr(&format!("(register :{})", name))
                .with_context(|| "Invalid name to register client process")?;
            client
                .request(reg_req)
                .await
                .with_context(|| "Failed to register client process")?;
        }

        if let Some(services) = args.get_many::<String>("bind_service") {
            for s in services {
                let reg_req = Form::from_expr(&format!("(bind_srv :{})", s))
                    .with_context(|| format!("Invalid service name: {}", s))?;
                client
                    .request(reg_req)
                    .await
                    .with_context(|| format!("Failed to bind_srv to {}", s))?;
            }
        }

        let file = open_file(
            args.get_one::<String>("file")
                .expect("file has a default value"),
        )?;

        let format = args
            .get_one::<Format>("format")
            .expect("format has a default value");

        if let Some(cmd) = args.get_one::<String>("command") {
            run_cmd(&client, cmd).await
        } else if let Some(file) = file {
            run_file(&client, format, file).await
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
        res = run => res,
        _ = client.closed() => Err(anyhow::anyhow!("Connection closed")),
    }
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
        .arg(arg!(format: --format <FORMAT> "Sets format of output")
             .default_value("default")
             .value_parser(EnumValueParser::<Format>::new())
        )
        .arg(arg!(name: -n --name <NAME> "Registers client process for this connection as NAME"))
        .arg(arg!(bind_service: -b --bind <NAME> "Binds client process to service named NAME")
             .action(ArgAction::Append))
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
        Ok(c) => {
            println!("{}", c);
            Ok(())
        }
        Err(e) => Err(anyhow::anyhow!("{e}")),
    }
}

/// Run a script file
async fn run_file(client: &Client, format: &Format, file: Box<dyn Read>) -> Result<()> {
    let mut f = BufReader::new(file);
    let mut line = String::new();
    let mut lineno = 0;
    loop {
        match f.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => (),
            Err(e) => return Err(e).with_context(|| "Error reading file"),
        }

        lineno += 1;

        let f = match lyric::parse(&line) {
            Ok(f) => f,
            Err(lyric::Error::IncompleteExpression(_)) => {
                continue;
            }
            Err(e) => {
                return Err(anyhow::anyhow!("{}: {} - {}", lineno, e, line.trim_end()));
            }
        };

        if *format == Format::Editor {
            print!("{}", line);
        }

        line.clear();

        let resp = client.request(f).await?;
        match resp.contents {
            Ok(c) if *format == Format::Editor => println!("# => {}", c),
            Ok(c) => println!("{}", c),
            Err(e) => return Err(anyhow::anyhow!("{e}")),
        }
    }

    if !line.trim().is_empty() && !line.trim().starts_with('#') {
        if let Err(e) = lyric::parse(&line) {
            return Err(anyhow::anyhow!("{}: {} - {}", lineno, e, line.trim()));
        }
    }

    Ok(())
}

// TODO: Test case for executing from stdin
// TODO: Test case for executing from REPL
// TODO: Test case for executing from -c CMD
// TODO: Test case for --format=editor
// TODO: Test case for --name=SRV_NAME
// TODO: Test case for incomplete expressions
// TODO: Test case for incomplete expressions that are comments
// TODO: Test case for --bind=SRV_NAME

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use vrs::Runtime;

    async fn runtime_client() -> (Runtime, Client) {
        let runtime = Runtime::new();
        let (client, runtime_conn) = Connection::pair().unwrap();
        runtime.handle_conn(runtime_conn).await.unwrap();
        (runtime, Client::new(client))
    }

    #[tokio::test]
    async fn run_cmd_succeeds_for_value() {
        let (_runtime, client) = runtime_client().await;
        run_cmd(&client, "(+ 20 22)").await.unwrap();
    }

    #[tokio::test]
    async fn run_cmd_propagates_evaluation_error() {
        let (_runtime, client) = runtime_client().await;
        let error = run_cmd(&client, "(undefined_function)").await.unwrap_err();

        assert!(error.to_string().contains("undefined_function"));
    }

    #[tokio::test]
    async fn run_cmd_propagates_async_error_without_closing_client() {
        let (_runtime, client) = runtime_client().await;
        let error = run_cmd(&client, "(publish :my_topic)").await.unwrap_err();

        assert!(error.to_string().contains("publish expects two arguments"));
        run_cmd(&client, "(+ 20 22)").await.unwrap();
    }

    #[tokio::test]
    async fn raw_block_string_executes_via_stdin_with_positional_arguments() {
        let (_runtime, client) = runtime_client().await;
        let expression = concat!(
            "(get (exec \"sh\" \"-s\" \"--\" \"argument with spaces\"\n",
            "           :stdin \"\"\"\n",
            "           printf '<%s>\\n' \"$1\"\n",
            "           printf '%s\\n' 'C:\\tmp \"quoted\"'\n",
            "           \"\"\") :stdout)",
        );

        let response = client
            .request(lyric::parse(expression).unwrap())
            .await
            .unwrap();
        assert_eq!(
            response.contents.unwrap(),
            lyric::Form::string("<argument with spaces>\nC:\\tmp \"quoted\"\n")
        );
    }

    #[test]
    fn all_repository_lyric_scripts_parse() {
        let scripts_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../scripts");
        for entry in std::fs::read_dir(scripts_dir).unwrap() {
            let entry = entry.unwrap();
            if !entry.file_type().unwrap().is_file() {
                continue;
            }
            let path = entry.path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("ll") {
                continue;
            }

            let source = std::fs::read_to_string(&path).unwrap();
            let mut pending = String::new();
            for (index, line) in source.split_inclusive('\n').enumerate() {
                pending.push_str(line);
                match lyric::parse(&pending) {
                    Ok(_) => pending.clear(),
                    Err(lyric::Error::IncompleteExpression(_)) => {}
                    Err(error) => panic!("{}:{}: {error}", path.display(), index + 1),
                }
            }

            assert!(
                pending.trim().is_empty() || pending.trim().starts_with('#'),
                "{} ended with an incomplete expression: {}",
                path.display(),
                pending.trim()
            );
        }
    }
}
