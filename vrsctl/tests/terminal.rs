//! Real CLI/PTY checks against an isolated daemon.
//! Run `cargo build -p vrsd` before `cargo test -p vrsctl --test terminal`.
//! Use the same profile/target directory for both commands.

#![cfg(unix)]

mod support;

use std::fs;
use std::io;
use std::path::Path;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{ensure, Result};
use assert_cmd::assert::OutputAssertExt;
use expectrl::Expect;
use support::{TestRuntime, TIMEOUT};

const VALUE: &str = "((:name :echo :node \"alpha\" :interface ((ping x) (pong y))) (:name :clock :interface ((now))))";
const PRETTY: &str = "((:name :echo\n  :node \"alpha\"\n  :interface ((ping x) (pong y)))\n (:name :clock :interface ((now))))\n";
const DEFAULT_PRETTY: &str = "((:name :echo :node \"alpha\" :interface ((ping x) (pong y)))\n (:name :clock :interface ((now))))\n";

#[test]
fn command_terminal_default_width_and_compact_override() -> Result<()> {
    let runtime = TestRuntime::new()?;
    let expression = format!("'{VALUE}");
    for (args, expected) in [
        (vec!["-c", expression.as_str()], DEFAULT_PRETTY.to_owned()),
        (
            vec!["--format", "compact", "-c", &expression],
            format!("{VALUE}\n"),
        ),
    ] {
        let mut terminal = runtime.terminal(&args, false, 40)?;
        terminal.expect(&expected)?;
        terminal.success()?;
    }
    Ok(())
}

#[test]
fn pipes_files_and_explicit_width() -> Result<()> {
    let runtime = TestRuntime::new()?;
    let expression = format!("'{VALUE}");
    assert_eq!(runtime.pipe(&["-c", &expression]), format!("{VALUE}\n"));
    assert_eq!(
        runtime.pipe(&["--format", "pretty", "--width", "40", "-c", &expression]),
        PRETTY
    );
    assert_eq!(
        runtime.pipe_input(&[], Some(&expression)),
        format!("{VALUE}\n")
    );
    let source = runtime.directory.path().join("values.ll");
    fs::write(&source, &expression)?;
    let source = source.to_str().unwrap();
    assert_eq!(runtime.pipe(&[source]), format!("{VALUE}\n"));
    let mut terminal = runtime.terminal(&[source], false, 40)?;
    terminal.expect(DEFAULT_PRETTY)?;
    terminal.success()?;
    let mut terminal = runtime.terminal(&["--width", "200", "-c", &expression], false, 40)?;
    terminal.expect(&format!("{VALUE}\n"))?;
    terminal.success()?;
    Ok(())
}

#[test]
fn pretty_builtin_raw_strings_and_editor_comments() -> Result<()> {
    let runtime = TestRuntime::new()?;
    let expression = format!("'{VALUE}");
    assert_eq!(
        runtime.pipe(&["--raw", "-c", &format!("(pretty {expression} 40)")]),
        PRETTY
    );
    let string = r#""one\n\"two\"""#;
    assert_eq!(
        runtime.pipe(&["--format", "pretty", "-c", string]),
        format!("{string}\n")
    );
    let transcript = runtime.pipe(&["--format", "editor", "--width", "45", "-c", &expression]);
    let mut lines = transcript.lines();
    assert_eq!(lines.next(), Some(expression.as_str()));
    assert!(lines.all(|line| line.starts_with("# ")));
    Ok(())
}

#[test]
fn default_width_is_90_in_builtin_pipes_and_terminals() -> Result<()> {
    let runtime = TestRuntime::new()?;
    let value = format!("({})", ["1234567890"; 8].join(" "));
    assert_eq!(value.len(), 89);
    assert_eq!(
        runtime.pipe(&["--raw", "-c", &format!("(pretty '{value})")]),
        format!("{value}\n")
    );
    assert_eq!(
        runtime.pipe(&["--format", "pretty", "-c", &format!("'{value}")]),
        format!("{value}\n")
    );
    for columns in [40, 200] {
        let mut terminal = runtime.terminal(&["-c", &format!("'{value}")], false, columns)?;
        terminal.expect(&format!("{value}\n"))?;
        terminal.success()?;
    }
    Ok(())
}

#[test]
fn repl_pretty_and_compact() -> Result<()> {
    let runtime = TestRuntime::new()?;
    for (args, expected) in [
        (vec![], DEFAULT_PRETTY.to_owned()),
        (vec!["--format", "compact"], format!("{VALUE}\n")),
    ] {
        let mut terminal = runtime.terminal(&args, true, 40)?;
        terminal.expect("vrs> ")?;
        terminal.session.send_line(format!("'{VALUE}"))?;
        terminal.expect(&expected)?;
        terminal.expect("vrs> ")?;
    }
    Ok(())
}

#[test]
fn quotation_round_trips_through_client_and_runtime() -> Result<()> {
    let runtime = TestRuntime::new()?;
    let source =
        "(let ((name 'focus_window) (window '(:id 7))) `(continue_call ',name '(,window)))";
    assert_eq!(
        runtime.pipe(&["-c", source]),
        "(continue_call 'focus_window '((:id 7)))\n"
    );
    assert_eq!(runtime.pipe(&["-c", "'(unquote @name)"]), ", @name\n");
    assert_eq!(
        runtime.pipe(&["-c", &format!("(eq? (read (pretty {source} 8)) {source})")]),
        "true\n"
    );
    assert_eq!(
        runtime.pipe(&["-c", "(let ((xs '(1 2))) `(a ,@xs))"]),
        "(a 1 2)\n"
    );
    Ok(())
}

#[test]
fn macros_expand_as_data_and_work_across_file_requests() -> Result<()> {
    let runtime = TestRuntime::new()?;
    assert_eq!(
        runtime.pipe(&["-c", "(macroexpand_1 '(when! true (missing)))"]),
        "(if true (begin (missing)) nil)\n"
    );
    let source = "(defmacro plus_one (x) `(+ ,x 1))\n(plus_one! 41)\n";
    assert_eq!(runtime.pipe_input(&[], Some(source)), "plus_one\n42\n");
    assert_eq!(
        runtime.pipe(&["-c", "(list (err? (try (plus_one! 1))) (when! true 42))"]),
        "(true 42)\n"
    );
    Ok(())
}

#[test]
fn service_expansion_contains_direct_match_and_round_trips() -> Result<()> {
    let runtime = TestRuntime::new()?;
    let source = "(begin
      (defn! echo (x) x)
      (def exports '(echo))
      (def code (macroexpand_1 '(srv! :test :interface exports)))
      (list (pretty code) (eq? code (read (pretty code)))
            (contains? (ls_srv) :test)))";
    let output = runtime.pipe(&["-c", source]);
    assert!(output.contains("((:echo x) (echo x))"), "{output}");
    assert!(
        output.contains(r#"(_ '(:err \"Unrecognized message\"))"#),
        "{output}"
    );
    assert!(!output.contains("service_dispatch"), "{output}");
    assert!(output.ends_with(" true false)\n"), "{output}");
    Ok(())
}

#[test]
fn init_service_and_nested_service_macros() -> Result<()> {
    let runtime = TestRuntime::new()?;
    let source = "(begin
      (bind_srv :test_factory)
      (start_child :test_child 42)
      (bind_srv :test_child)
      (read_value))";
    assert_eq!(runtime.pipe(&["-c", source]), "42\n");
    Ok(())
}

#[test]
fn subscriptions_once_follow_and_clear() -> Result<()> {
    let runtime = TestRuntime::new()?;
    for (index, mode) in [None, Some("-f"), Some("-F")].into_iter().enumerate() {
        let topic = format!("format_test_{index}");
        let mut args = vec!["-s", topic.as_str()];
        args.extend(mode);
        let mut terminal = runtime.terminal(&args, false, 40)?;
        let deadline = Instant::now() + TIMEOUT;
        // Publish until output arrives instead of assuming the subscriber has
        // registered after an arbitrary startup delay. Matching retains output.
        while !terminal
            .session
            .is_matched(DEFAULT_PRETTY.replace('\n', "\r\n"))?
        {
            ensure!(Instant::now() < deadline, "subscriber did not become ready");
            runtime.pipe(&["-c", &format!("(publish :{topic} '{VALUE})")]);
            thread::sleep(Duration::from_millis(30));
        }
        terminal.expect(DEFAULT_PRETTY)?;
        if mode.is_some() {
            runtime.pipe(&["-c", &format!("(publish :{topic} '(:second 42))")]);
            terminal.expect("(:second 42)\n")?;
            terminal.assert_alive()?;
            terminal.resize(200)?;
            runtime.pipe(&["-c", &format!("(publish :{topic} '{VALUE})")]);
            terminal.expect(DEFAULT_PRETTY)?;
        } else {
            terminal.success()?;
        }
    }
    Ok(())
}

#[test]
fn emacs_evaluation_against_test_runtime() -> Result<()> {
    let runtime = TestRuntime::new()?;
    let mut command = assert_cmd::Command::new("emacs");
    command
        .args(["-Q", "--batch", "-L"])
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../emacs"))
        .args(["-l", "vrs-mode-tests", "-f", "ert-run-tests-batch-and-exit"])
        .env("VRS_TEST_VRSCTL", runtime.emacs_command())
        .timeout(Duration::from_secs(20));
    match command.output() {
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            eprintln!("skipping Emacs evaluation: Emacs is not installed");
        }
        result => {
            let output = result?;
            eprint!("{}", String::from_utf8_lossy(&output.stderr));
            output.assert().success();
        }
    }
    Ok(())
}

#[test]
fn dbg_views_share_real_recording_filters_and_editor_source_origins() -> Result<()> {
    let runtime = TestRuntime::new()?;
    let source = "(defn! twice (x) (+ x x))\n(dbg! (map '(2 3) twice))";
    let request =
        serde_json::json!({"source":source,"file":"/tmp/observe.ll","line":20,"column":1});
    runtime.pipe_input(&["--session"], Some(&format!("{request}\n")));
    let transcript = runtime.pipe(&["dbg", "--once", "--all", "--details"]);
    ensure!(transcript.contains("(map '(2 3) twice)"), "{transcript}");
    ensure!(transcript.contains("/tmp/observe.ll:21:7"), "{transcript}");
    ensure!(transcript.contains("# arg 1: 2"), "{transcript}");
    let json = runtime.pipe(&["dbg", "--once", "--json", "--at", "observe.ll:20:18"]);
    // Correct source location (the inner + starts at column 18 here).
    let history: serde_json::Value = serde_json::from_str(&json)?;
    let records = history["records"].as_array().unwrap();
    ensure!(
        records
            .iter()
            .filter(|r| r["site"]["form"] == "(+ x x)")
            .count()
            == 2,
        "{json}"
    );
    Ok(())
}
