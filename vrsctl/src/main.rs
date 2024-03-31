mod dbg;
mod editor;
mod output;
mod repl;
mod session;
mod watch;

use anyhow::{Context, Result};
use clap::builder::EnumValueParser;
use clap::{arg, command, ArgAction, ArgGroup};
use output::{Format, Output};

use std::fs::File;
use std::io::{self, BufRead, BufReader, IsTerminal, Read, Write};
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::str::FromStr;
use tokio::net::UnixStream;
use tracing::debug;
use vrs::{Client, Connection, Form, KeywordId};

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
        if let Some(node) = args.get_one::<String>("node") {
            client.select_node(node).await?.contents?;
        }
        if let Some(dbg) = args.subcommand_matches("dbg") {
            return dbg::run(
                &client,
                dbg,
                args.get_one::<NonZeroUsize>("width")
                    .map(|n| n.get())
                    .unwrap_or(100),
            )
            .await;
        }
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

        let format = *args
            .get_one::<Format>("format")
            .expect("format has a default value");
        let output = Output::new(
            format,
            args.get_one::<NonZeroUsize>("width")
                .map(|width| width.get()),
            args.get_flag("raw"),
        );
        let mut stdout = io::stdout();

        if args.get_flag("session") {
            session::run(&client, &output, BufReader::new(io::stdin()), &mut stdout).await
        } else if let Some(cmd) = args.get_one::<String>("command") {
            run_cmd(&client, cmd, &output, &mut stdout).await
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
                &output,
            )
            .await
        } else {
            match open_file(
                args.get_one::<String>("file")
                    .expect("file has a default value"),
            )? {
                Some(file) => {
                    let name = args.get_one::<String>("file").unwrap();
                    let origin = if name == "-" {
                        "<stdin>".into()
                    } else {
                        std::fs::canonicalize(name)?.to_string_lossy().into_owned()
                    };
                    run_file_at(&client, &output, file, &mut stdout, &origin, 1, 1).await
                }
                None => repl::run(&client, &output).await,
            }
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
        .arg(arg!(node: --node <NODE> "Evaluate on a named connected node; files are read by this client").global(true))
        .subcommand(dbg::command())
        .arg(arg!(file: [FILE] "If present, executes contents of FILE")
             .default_value("-")
             .conflicts_with_all(["command", "subscribe", "session"]))
        .arg(arg!(session: --session "Keep a connection for editor requests: one JSON object per stdin line")
             .long_help("Keep a connection for editor requests. Each stdin line is JSON with source (required), format, width, raw, and literal fields. Literal results quote lists and symbols for source retention. Each stdout line is JSON with ok and output or error fields. Definitions persist until the connection closes; evaluation errors leave the session open."))
        .arg(arg!(command: -c --command <EXPR> "If present, EXPR is sent as request, then program exits"))
        .arg(arg!(subscribe: -s --subscribe <TOPIC> "If present, watches a specific topic for data"))
        .group(ArgGroup::new("main")
               .args(["command", "subscribe", "session"])
               .required(false))
        .arg(arg!(follow: -f --follow "If present, continues polling subscription after first topic update")
             .requires("subscribe"))
        .arg(arg!(follow_clear: -F --followclear "Like --follow, but clears screen after each value")
            .requires("subscribe"))
        .arg(arg!(format: --format <FORMAT> "Sets format of output")
             .default_value("default")
             .value_parser(EnumValueParser::<Format>::new())
        )
        .arg(arg!(width: --width <COLUMNS> "Target width for pretty/editor output (default 90); atoms may exceed it")
             .global(true).value_parser(clap::value_parser!(NonZeroUsize)))
        .arg(arg!(raw: --raw "Print top-level strings verbatim; nested strings remain quoted"))
        .arg(arg!(name: -n --name <NAME> "Registers client process for this connection as NAME"))
        .arg(arg!(bind_service: -b --bind <NAME> "Binds client process to service named NAME")
             .action(ArgAction::Append))
        .arg(
            arg!(socket: -S --socket <SOCKET> "Path to unix socket for vrsd")
                .default_value(vrs::runtime_socket().into_os_string()).global(true),
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
async fn run_cmd(
    client: &Client,
    cmd: &str,
    output: &Output,
    writer: &mut impl Write,
) -> Result<()> {
    lyric::parse(cmd)?;
    let resp = client
        .request(lyric::source::request(cmd, "<command>", 1, 1))
        .await?;
    match resp.contents {
        Ok(c) => {
            output.write(writer, &c, cmd)?;
            Ok(())
        }
        Err(e) => Err(anyhow::anyhow!("{e}")),
    }
}

/// Test helper for source without a named origin.
#[cfg(test)]
async fn run_file(
    client: &Client,
    output: &Output,
    file: Box<dyn Read>,
    writer: &mut impl Write,
) -> Result<()> {
    run_file_at(client, output, file, writer, "<stdin>", 1, 1).await
}

async fn run_file_at(
    client: &Client,
    output: &Output,
    file: Box<dyn Read>,
    writer: &mut impl Write,
    origin: &str,
    first_line: usize,
    first_column: usize,
) -> Result<()> {
    let mut f = BufReader::new(file);
    let mut line = String::new();
    let mut lineno = first_line - 1;
    let mut source_line = first_line;
    let mut source_column = first_column;
    loop {
        match f.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => (),
            Err(e) => return Err(e).with_context(|| "Error reading file"),
        }

        lineno += 1;

        match lyric::parse(&line) {
            Ok(_) => (),
            Err(lyric::Error::IncompleteExpression(_)) => {
                continue;
            }
            Err(e) => {
                return Err(anyhow::anyhow!("{}: {} - {}", lineno, e, line.trim_end()));
            }
        };

        let resp = client
            .request(lyric::source::request(
                &line,
                origin,
                source_line,
                source_column,
            ))
            .await?;
        match resp.contents {
            Ok(c) => output.write(writer, &c, &line)?,
            Err(e) => return Err(anyhow::anyhow!("{e}")),
        }
        line.clear();
        source_line = lineno + 1;
        source_column = 1;
    }

    if !line.trim().is_empty() && !line.trim().starts_with('#') {
        if let Err(e) = lyric::parse(&line) {
            return Err(anyhow::anyhow!("{}: {} - {}", lineno, e, line.trim()));
        }
    }

    Ok(())
}

// TODO: Test case for --name=SRV_NAME
// TODO: Test case for incomplete expressions
// TODO: Test case for incomplete expressions that are comments
// TODO: Test case for --bind=SRV_NAME

#[cfg(test)]
mod tests {
    use super::*;
    use vrs::Runtime;

    async fn runtime_client() -> (Runtime, Client) {
        let runtime = Runtime::new("vrsctl");
        let (client, runtime_conn) = Connection::pair().unwrap();
        runtime.handle_conn(runtime_conn).await.unwrap();
        (runtime, Client::new(client))
    }

    #[test]
    fn cli_accepts_output_options_in_all_modes_and_validates_width() {
        for args in [
            vec!["vrsctl", "--socket", "/tmp/vrs.sock", "dbg"],
            vec!["vrsctl", "dbg", "--socket", "/tmp/vrs.sock", "--all"],
            vec![
                "vrsctl",
                "--width",
                "120",
                "dbg",
                "--web",
                "--filter",
                "file::example.ll",
            ],
        ] {
            assert_eq!(
                cli().try_get_matches_from(args).unwrap().subcommand_name(),
                Some("dbg")
            );
        }
        for mode in [
            vec![],
            vec!["-c", "(ls_srv)"],
            vec!["example.ll"],
            vec!["-"],
            vec!["-s", "topic", "-f"],
            vec!["-s", "topic", "-F"],
            vec!["--session"],
        ] {
            let mut args = vec!["vrsctl", "--format", "pretty", "--width", "40", "--raw"];
            args.extend(mode);
            cli().try_get_matches_from(args).unwrap();
        }
        for width in ["0", "-1", "abc"] {
            assert!(cli()
                .try_get_matches_from(["vrsctl", "--width", width])
                .is_err());
        }
        assert!(cli()
            .try_get_matches_from(["vrsctl", "example.ll", "-s", "topic"])
            .is_err());
        for arguments in [
            vec!["vrsctl", "--session", "-c", "42"],
            vec!["vrsctl", "--session", "example.ll"],
            vec!["vrsctl", "--session", "-s", "topic"],
        ] {
            assert!(cli().try_get_matches_from(arguments).is_err());
        }
    }

    #[tokio::test]
    async fn command_and_script_results_share_the_output_policy() {
        let (_runtime, client) = runtime_client().await;
        let expr = "'((1 2) (3 4))";
        for (format, expected) in [
            (Format::Compact, "((1 2) (3 4))\n"),
            (Format::Pretty, "((1 2)\n (3 4))\n"),
            (
                Format::Editor,
                "'((1 2) (3 4))\n# => ((1 2)\n#     (3 4))\n",
            ),
        ] {
            let output = Output::new(
                format,
                Some(if format == Format::Editor { 16 } else { 11 }),
                false,
            );
            let mut command = Vec::new();
            run_cmd(&client, expr, &output, &mut command).await.unwrap();
            assert_eq!(String::from_utf8(command).unwrap(), expected);
            let mut script = Vec::new();
            run_file(
                &client,
                &output,
                Box::new(std::io::Cursor::new(expr)),
                &mut script,
            )
            .await
            .unwrap();
            assert_eq!(String::from_utf8(script).unwrap(), expected);
        }
    }

    #[tokio::test]
    async fn editor_transcript_keeps_multiline_results_in_comments() {
        let (_runtime, client) = runtime_client().await;
        let source = "# sample\n(pretty '(1 2 3) 1)\n(+ 20 22)";
        let output = Output::new(Format::Editor, Some(80), true);
        let mut buf = Vec::new();
        run_file(
            &client,
            &output,
            Box::new(std::io::Cursor::new(source)),
            &mut buf,
        )
        .await
        .unwrap();
        let transcript = String::from_utf8(buf).unwrap();
        assert_eq!(
            transcript,
            "# sample\n(pretty '(1 2 3) 1)\n# => (1\n#     2\n#     3)\n(+ 20 22)\n# => 42\n"
        );
        assert_eq!(
            lyric::parse_script(&transcript).unwrap(),
            lyric::parse_script(source).unwrap()
        );
    }

    #[tokio::test]
    async fn service_registry_and_pretty_builtin_have_the_same_readable_output() {
        let (_runtime, client) = runtime_client().await;
        client
            .request(
                lyric::parse(
                    "(begin (defn! ping (x) x) (register :format_example :interface '(ping)))",
                )
                .unwrap(),
            )
            .await
            .unwrap()
            .contents
            .unwrap();
        let mut normal = Vec::new();
        run_cmd(
            &client,
            "(ls_srv)",
            &Output::new(Format::Pretty, Some(25), false),
            &mut normal,
        )
        .await
        .unwrap();
        let mut explicit = Vec::new();
        run_cmd(
            &client,
            "(pretty (ls_srv) 25)",
            &Output::new(Format::Compact, None, true),
            &mut explicit,
        )
        .await
        .unwrap();
        assert_eq!(normal, explicit);
        let text = String::from_utf8(normal).unwrap();
        assert!(text.contains(":name :format_example"));
        assert!(text.starts_with("(:format_example\n ("), "{text}");
        assert!(text.contains("\n  :pid <vrsctl:"), "{text}");
        assert!(text.contains(":interface"));
    }

    #[tokio::test]
    async fn run_cmd_succeeds_for_value() {
        let (_runtime, client) = runtime_client().await;
        run_cmd(
            &client,
            "(+ 20 22)",
            &Output::new(Format::Default, None, false),
            &mut Vec::new(),
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn run_cmd_propagates_evaluation_error() {
        let (_runtime, client) = runtime_client().await;
        let error = run_cmd(
            &client,
            "(undefined_function)",
            &Output::new(Format::Default, None, false),
            &mut Vec::new(),
        )
        .await
        .unwrap_err();

        assert!(error.to_string().contains("undefined_function"));
    }

    #[tokio::test]
    async fn run_cmd_propagates_async_error_without_closing_client() {
        let (_runtime, client) = runtime_client().await;
        let error = run_cmd(
            &client,
            "(publish :my_topic)",
            &Output::new(Format::Default, None, false),
            &mut Vec::new(),
        )
        .await
        .unwrap_err();

        assert!(error.to_string().contains("publish expects two arguments"));
        run_cmd(
            &client,
            "(+ 20 22)",
            &Output::new(Format::Default, None, false),
            &mut Vec::new(),
        )
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn raw_block_string_survives_client_serialization() {
        let (_runtime, client) = runtime_client().await;
        let source = r#""""
            first "quoted" line
            C:\tmp
            """"#;
        let response = client.request(lyric::parse(source).unwrap()).await.unwrap();
        assert_eq!(
            response.contents.unwrap(),
            lyric::Form::string("first \"quoted\" line\nC:\\tmp\n")
        );
    }
}
