//! Line-delimited editor requests on one runtime connection. Source and results
//! are JSON strings, so multiline Lyric and printed values need no delimiters.
use crate::output::{Format, Output};
use anyhow::{ensure, Result};
use lyric::Form;
use serde::Deserialize;
use serde_json::{json, Value};
use std::io::{BufRead, Cursor, Write};
use std::num::NonZeroUsize;
use vrs::Client;

#[derive(Deserialize)]
struct Request {
    source: String,
    file: Option<String>,
    line: Option<NonZeroUsize>,
    column: Option<NonZeroUsize>,
    format: Option<Format>,
    width: Option<NonZeroUsize>,
    raw: Option<bool>,
    literal: Option<bool>,
}

pub(crate) async fn run(
    client: &Client,
    defaults: &Output,
    input: impl BufRead + Send + 'static,
    writer: &mut impl Write,
) -> Result<()> {
    // A plain reader thread lets the client exit on daemon disconnect even
    // while stdin is idle. Tokio's blocking stdin task would delay shutdown.
    let (sender, mut lines) = tokio::sync::mpsc::channel(1);
    std::thread::spawn(move || {
        for line in input.lines() {
            if sender.blocking_send(line).is_err() {
                break;
            }
        }
    });
    while let Some(line) = lines.recv().await {
        let reply = respond(client, defaults, &line?).await;
        serde_json::to_writer(&mut *writer, &reply)?;
        writeln!(writer)?;
        writer.flush()?;
    }
    Ok(())
}

async fn respond(client: &Client, defaults: &Output, line: &str) -> Value {
    match evaluate(client, defaults, line).await {
        Ok((output, literal)) => {
            let mut reply = json!({"ok": true, "output": output});
            if literal {
                reply["literal"] = json!(true);
            }
            reply
        }
        Err(error) => json!({"ok": false, "error": error.to_string()}),
    }
}

async fn evaluate(client: &Client, defaults: &Output, line: &str) -> Result<(String, bool)> {
    let request: Request = serde_json::from_str(line)?;
    let literal = request.literal.unwrap_or(false);
    let output = Output::new(
        request.format.unwrap_or(defaults.format),
        request.width.map(NonZeroUsize::get).or(defaults.width),
        !literal && request.raw.unwrap_or(defaults.raw),
    );
    ensure!(
        !literal || output.format != Format::Editor,
        "Literal results cannot be combined with an editor transcript"
    );
    let mut text = Vec::new();
    let origin = request.file.as_deref().unwrap_or("<editor>");
    let line = request.line.map(NonZeroUsize::get).unwrap_or(1);
    let column = request.column.map(NonZeroUsize::get).unwrap_or(1);
    if output.format == Format::Editor {
        crate::run_file_at(
            client,
            &output,
            Box::new(Cursor::new(request.source)),
            &mut text,
            origin,
            line,
            column,
        )
        .await?;
    } else {
        let value = client
            .request(lyric::source::request(
                &request.source,
                origin,
                line,
                column,
            ))
            .await?
            .contents?;
        let value = if literal { retain_value(value)? } else { value };
        output.write(&mut text, &value, &request.source)?;
    }
    Ok((String::from_utf8(text)?, literal))
}

fn retain_value(value: Form) -> Result<Form> {
    // Opaque runtime values can print as plausible source without preserving
    // their meaning. Validate the entire value, including nested elements.
    ensure!(
        Form::from_expr(&value.to_string()).is_ok_and(|parsed| parsed == value),
        "This value cannot be retained as Lyric source"
    );
    Ok(match value {
        Form::List(_) | Form::Symbol(_) => Form::List(vec![Form::symbol("quote"), value]),
        _ => value,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use vrs::{Connection, Runtime};

    #[tokio::test]
    async fn literal_results_round_trip_without_repeating_evaluation() {
        let runtime = Runtime::new("literal-session");
        let (local, remote) = Connection::pair().unwrap();
        runtime.handle_conn(remote).await.unwrap();
        let client = Client::new(local);
        let defaults = Output::new(Format::Pretty, Some(20), false);
        for source in [
            "'(:todo :title \"Read\" :tags (a b))",
            "'symbol",
            "''(open_url \"https://example.com\")",
            "'()",
            "\"東京\\n\\\"quoted\\\"\"",
            ":ready",
            "true",
            "nil",
            "42",
        ] {
            let expected = client
                .request(Form::from_expr(source).unwrap())
                .await
                .unwrap()
                .contents
                .unwrap();
            let reply = respond(
                &client,
                &defaults,
                &json!({"source": source, "literal": true, "raw": true}).to_string(),
            )
            .await;
            assert_eq!(reply["ok"], true, "{source}: {reply}");
            let retained = Form::from_expr(reply["output"].as_str().unwrap()).unwrap();
            assert_eq!(
                client.request(retained).await.unwrap().contents.unwrap(),
                expected
            );
        }
        let reply = respond(
            &client,
            &defaults,
            &json!({
                "source": "(def count 0)\n(set count (+ count 1))\n(list count)",
                "literal": true
            })
            .to_string(),
        )
        .await;
        assert_eq!(reply["output"], "'(1)\n");
        let count = respond(&client, &defaults, r#"{"source":"count"}"#).await;
        assert_eq!(count["output"], "1\n");
        for source in ["(self)", "(fn () 1)", "(list :nested (self))"] {
            let reply = respond(
                &client,
                &defaults,
                &json!({"source": source, "literal": true}).to_string(),
            )
            .await;
            assert_eq!(reply["ok"], false, "{source}: {reply}");
            assert!(reply["error"]
                .as_str()
                .unwrap()
                .contains("cannot be retained"));
        }
        let reply = respond(
            &client,
            &defaults,
            &json!({
                "source": "(set count 99)", "literal": true, "format": "editor"
            })
            .to_string(),
        )
        .await;
        assert_eq!(reply["ok"], false);
        let count = respond(&client, &defaults, r#"{"source":"count"}"#).await;
        assert_eq!(count["output"], "1\n");
    }

    #[tokio::test]
    async fn requests_share_definitions_and_macros_and_recover_from_errors() {
        let runtime = Runtime::new("session");
        let (local, remote) = Connection::pair().unwrap();
        runtime.handle_conn(remote).await.unwrap();
        let client = Client::new(local);
        let defaults = Output::new(Format::Pretty, None, false);
        let sources = [
            "(def exports '(echo))\n(defn! echo (x) x)\n(defmacro answer () 42)",
            "(macroexpand_1 '(srv! :example :interface exports))",
            "(macroexpand_1 '(answer!))",
            "(error \"failed\")",
            "(",
            "(echo 42)",
        ];
        let mut input = String::from("invalid json\n");
        for source in sources {
            input.push_str(&json!({"source": source}).to_string());
            input.push('\n');
        }
        let mut output = vec![];
        run(&client, &defaults, Cursor::new(input), &mut output)
            .await
            .unwrap();
        let replies: Vec<Value> = String::from_utf8(output)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(replies.len(), 7);
        for index in [0, 4, 5] {
            assert_eq!(replies[index]["ok"], false);
        }
        assert!(replies[2]["output"].as_str().unwrap().contains("register"));
        assert_eq!(replies[3]["output"], "42\n");
        assert_eq!(replies[6]["output"], "42\n");
    }

    #[tokio::test]
    async fn formatting_is_per_request_and_multiline_text_stays_one_record() {
        let runtime = Runtime::new("session");
        let (local, remote) = Connection::pair().unwrap();
        runtime.handle_conn(remote).await.unwrap();
        let client = Client::new(local);
        let defaults = Output::new(Format::Compact, None, false);
        for (request, expected) in [
            (
                json!({"source": "(def text \"東京\\nhello\")", "raw": true}),
                "東京\nhello\n",
            ),
            (
                json!({"source": "text", "raw": false}),
                "\"東京\\nhello\"\n",
            ),
            (
                json!({"source": "'((1 2) (3 4))", "format": "pretty", "width": 10}),
                "((1 2)\n (3 4))\n",
            ),
            (
                json!({"source": "(+ 1 2)\n(+ 3 4)", "format": "editor"}),
                "(+ 1 2)\n# => 3\n(+ 3 4)\n# => 7\n",
            ),
        ] {
            let reply = respond(&client, &defaults, &request.to_string()).await;
            assert_eq!(reply, json!({"ok": true, "output": expected}));
            assert!(!serde_json::to_string(&reply).unwrap().contains('\n'));
        }
        assert_eq!(
            respond(&client, &defaults, r#"{"source":"42","width":0}"#).await["ok"],
            false
        );
    }
}
