//! Shared dbg viewer model and CLI source transcript.
mod web;
use anyhow::{Context, Result};
use clap::{Arg, ArgAction, ArgMatches, Command};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    io::{self, Write},
    time::Duration,
};
use vrs::{
    debug::{Record, Snapshot},
    Client,
};

pub fn command() -> Command {
    Command::new("dbg")
        .about("Inspect source-embedded dbg! observations (history, then live updates)")
        .arg(
            Arg::new("web")
                .long("web")
                .action(ArgAction::SetTrue)
                .help("Open the local browser viewer"),
        )
        .arg(
            Arg::new("no_open")
                .long("no-open")
                .requires("web")
                .action(ArgAction::SetTrue)
                .help("Print the viewer URL without opening it"),
        )
        .arg(
            Arg::new("once")
                .long("once")
                .conflicts_with("web")
                .action(ArgAction::SetTrue)
                .help("Print the current snapshot and exit"),
        )
        .arg(
            Arg::new("json")
                .long("json")
                .conflicts_with("web")
                .action(ArgAction::SetTrue)
                .help("Output the complete structured snapshot (one JSON object per update)"),
        )
        .arg(
            Arg::new("all")
                .long("all")
                .action(ArgAction::SetTrue)
                .help("Include nested calls and callbacks; expand all in the browser"),
        )
        .arg(
            Arg::new("details")
                .long("details")
                .action(ArgAction::SetTrue)
                .help("Include actual arguments and full bounded results"),
        )
        .arg(
            Arg::new("file")
                .long("file")
                .value_name("PATH")
                .help("Filter by file path substring"),
        )
        .arg(
            Arg::new("at")
                .long("at")
                .value_name("FILE:LINE[:COLUMN]")
                .help("Show history at a source location"),
        )
        .arg(
            Arg::new("expr")
                .long("expr")
                .value_name("TEXT")
                .help("Filter by source expression substring"),
        )
        .arg(
            Arg::new("run")
                .long("run")
                .value_name("ID")
                .help("Inspect one run (ID prefix accepted)"),
        )
        .arg(
            Arg::new("call")
                .long("call")
                .value_name("ID")
                .help("Inspect one call and its descendants (ID prefix accepted)"),
        )
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Filter {
    pub file: String,
    pub at: String,
    pub expr: String,
    pub run: String,
    pub call: String,
}
impl Filter {
    fn from_args(args: &ArgMatches) -> Self {
        let get = |key| args.get_one::<String>(key).cloned().unwrap_or_default();
        Self {
            file: get("file"),
            at: get("at"),
            expr: get("expr"),
            run: get("run"),
            call: get("call"),
        }
    }
    fn active(&self) -> bool {
        [&self.file, &self.at, &self.expr, &self.run, &self.call]
            .iter()
            .any(|s| !s.is_empty())
    }
    fn validate(&self) -> Result<()> {
        if !self.at.is_empty() {
            parse_location(&self.at)?;
        }
        Ok(())
    }
    fn matches(&self, r: &Record) -> bool {
        let s = &r.event.site;
        s.file.contains(&self.file)
            && s.form.contains(&self.expr)
            && r.event.run.starts_with(&self.run)
            && (self.at.is_empty()
                || parse_location(&self.at).is_ok_and(|(file, line, column)| {
                    (s.file == file || s.file.ends_with(&format!("/{file}")))
                        && s.line == line
                        && column.is_none_or(|c| s.column == c)
                }))
    }
    /// Preserve ancestors for context. Selecting a call includes its subtree.
    pub fn select(&self, snapshot: &Snapshot) -> Snapshot {
        let records: HashMap<_, _> = snapshot
            .records
            .iter()
            .map(|r| (r.event.id.as_str(), r))
            .collect();
        let within_call = |r: &Record| {
            let mut current = Some(r);
            for _ in 0..=snapshot.records.len() {
                let Some(r) = current else {
                    break;
                };
                if r.event.id.starts_with(&self.call) {
                    return true;
                }
                current = r
                    .event
                    .parent
                    .as_deref()
                    .and_then(|id| records.get(id).copied());
            }
            false
        };
        let mut selected = HashSet::new();
        for r in &snapshot.records {
            if self.matches(r) && (self.call.is_empty() || within_call(r)) {
                selected.insert(r.event.id.as_str());
                let mut parent = r.event.parent.as_deref();
                for _ in 0..snapshot.records.len() {
                    let Some(id) = parent else {
                        break;
                    };
                    if !selected.insert(id) {
                        break;
                    }
                    parent = records.get(id).and_then(|r| r.event.parent.as_deref());
                }
            }
        }
        Snapshot {
            records: snapshot
                .records
                .iter()
                .filter(|r| selected.contains(r.event.id.as_str()))
                .cloned()
                .collect(),
            ..snapshot.clone()
        }
    }
}
fn parse_location(value: &str) -> Result<(&str, usize, Option<usize>)> {
    let (before, last) = value
        .rsplit_once(':')
        .context("--at expects FILE:LINE[:COLUMN]")?;
    let last: usize = last
        .parse()
        .context("source line/column must be a positive integer")?;
    anyhow::ensure!(last > 0, "source line/column must be positive");
    if let Some((file, line)) = before.rsplit_once(':') {
        if let Ok(line) = line.parse::<usize>() {
            anyhow::ensure!(line > 0 && !file.is_empty(), "invalid source location");
            return Ok((file, line, Some(last)));
        }
    }
    anyhow::ensure!(!before.is_empty(), "source file must not be empty");
    Ok((before, last, None))
}

pub async fn snapshot(client: &Client) -> Result<Snapshot> {
    let response = client
        .request(lyric::parse("(dbg_history)")?)
        .await?
        .contents
        .context("The runtime needs dbg! support; rebuild/restart vrsd")?;
    Ok(Snapshot::from_form(response)?)
}

pub async fn run(client: &Client, args: &ArgMatches, width: usize) -> Result<()> {
    let filter = Filter::from_args(args);
    filter.validate()?;
    if args.get_flag("web") {
        return web::run(
            client,
            filter,
            args.get_flag("no_open"),
            args.get_flag("all"),
            args.get_flag("details"),
        )
        .await;
    }
    let mut subscription = client.subscribe(vrs::debug::TOPIC.into()).await?;
    let mut seen = HashMap::new();
    let mut counters = None;
    let mut last_cursor = None;
    loop {
        let all = snapshot(client).await?;
        let selected = filter.select(&all);
        if args.get_flag("json") {
            if last_cursor != Some((all.cursor, all.dropped, all.evicted)) {
                serde_json::to_writer(io::stdout(), &selected)?;
                println!();
                last_cursor = Some((all.cursor, all.dropped, all.evicted));
            }
        } else {
            if counters.is_none() && selected.records.is_empty() {
                eprintln!(
                    "{}",
                    if filter.active() {
                        "No observations match these filters."
                    } else {
                        "No dbg! observations yet. Evaluate a (dbg! …) block to start recording."
                    }
                );
            }
            if counters != Some((all.dropped, all.evicted)) && (all.dropped > 0 || all.evicted > 0)
            {
                eprintln!(
                    "dbg: {} observations dropped; {} records evicted from bounded history",
                    all.dropped, all.evicted
                );
            }
            counters = Some((all.dropped, all.evicted));
            let by_id: HashMap<_, _> = selected
                .records
                .iter()
                .map(|r| (r.event.id.as_str(), r))
                .collect();
            for record in &selected.records {
                let e = &record.event;
                let visible = args.get_flag("all")
                    || filter.active()
                    || e.kind == "scope"
                    || e.parent
                        .as_deref()
                        .and_then(|id| by_id.get(id))
                        .is_none_or(|p| p.event.kind == "scope");
                if !visible || seen.get(&e.id) == Some(&record.sequence) {
                    continue;
                }
                // Fast calls get one completed line; longer calls show a pending
                // line followed by completion with the same call ID.
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64;
                if e.status == "running"
                    && !args.get_flag("once")
                    && now.saturating_sub(e.started_ms) < 250
                {
                    continue;
                }
                print_record(&mut io::stdout(), record, width, args.get_flag("details"))?;
                seen.insert(e.id.clone(), record.sequence);
            }
            seen.retain(|id, _| all.records.iter().any(|r| r.event.id == *id));
            io::stdout().flush()?;
        }
        if args.get_flag("once") {
            return Ok(());
        }
        tokio::select! {
            _=tokio::signal::ctrl_c()=>return Ok(()),
            _=subscription.recv()=>(),
            _=tokio::time::sleep(Duration::from_millis(250))=>(),
        }
    }
}

fn safe(text: &str) -> String {
    text.chars()
        .flat_map(|ch| {
            if ch.is_control() && ch != '\n' {
                ch.escape_default().collect::<Vec<_>>()
            } else {
                vec![ch]
            }
        })
        .collect()
}
fn print_record(writer: &mut impl Write, r: &Record, width: usize, details: bool) -> Result<()> {
    let e = &r.event;
    let s = &e.site;
    let result = e
        .result
        .as_ref()
        .map(|r| r.text.as_str())
        .unwrap_or(&e.status);
    let filename = std::path::Path::new(&s.file)
        .file_name()
        .and_then(|p| p.to_str())
        .unwrap_or(&s.file);
    let location = if s.line > 0 {
        format!("{}:{}:{}", filename, s.line, s.column)
    } else {
        s.file.clone()
    };
    let note = if e.kind == "callback" {
        " · callback"
    } else if s.generated {
        " · generated"
    } else {
        ""
    };
    let meta = format!(
        "{} · {:.2}ms · {}{}{}",
        location,
        e.elapsed_us as f64 / 1000.,
        &e.id[..e.id.len().min(8)],
        note,
        if e.status == "error" {
            " · error"
        } else if e.status == "cancelled" {
            " · cancelled"
        } else {
            ""
        }
    );
    let line = format!("{}  # => {}  [{}]", s.form, result, meta);
    if line.chars().count() <= width && !line.contains('\n') {
        writeln!(writer, "{}", safe(&line))?;
    } else {
        let source = lyric::parse(&s.form)
            .map(|f| f.to_pretty_string(width))
            .unwrap_or_else(|_| s.form.clone());
        writeln!(writer, "{}  # {}", safe(&source), safe(&meta))?;
        let result = lyric::parse(result)
            .map(|f| f.to_pretty_string(width.saturating_sub(6)))
            .unwrap_or_else(|_| result.into());
        for (i, line) in result.lines().enumerate() {
            writeln!(
                writer,
                "# {}{}",
                if i == 0 { "=> " } else { "   " },
                safe(line)
            )?;
        }
    }
    if details {
        writeln!(writer, "# source {}:{}:{}", safe(&s.file), s.line, s.column)?;
        writeln!(
            writer,
            "# call {} · run {} · process {} · {}",
            e.id, e.run, r.process, e.status
        )?;
        if let Some(parent) = &e.parent {
            writeln!(writer, "# parent {parent}")?;
        }
        for (i, arg) in e.arguments.iter().enumerate() {
            writeln!(
                writer,
                "# arg {}: {}{}",
                i + 1,
                safe(&arg.text),
                if arg.truncated { " [truncated]" } else { "" }
            )?;
        }
        if e.arguments_truncated {
            writeln!(writer, "# additional arguments omitted")?;
        }
        if e.result.as_ref().is_some_and(|r| r.truncated) {
            writeln!(writer, "# result truncated")?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use vrs::{Connection, Runtime};
    async fn fixture() -> (Runtime, Client, Snapshot) {
        let runtime = Runtime::new("viewer");
        let (a, b) = Connection::pair().unwrap();
        runtime.handle_conn(b).await.unwrap();
        let client = Client::new(a);
        let source = "(defn! twice (x) (+ x x))\n(dbg! (map '(2 3) twice))\n(dbg! (twice 4))";
        client
            .request(lyric::source::request(source, "/tmp/example.ll", 1, 1))
            .await
            .unwrap()
            .contents
            .unwrap();
        let snapshot = snapshot(&client).await.unwrap();
        (runtime, client, snapshot)
    }
    #[tokio::test]
    async fn source_history_call_subtrees_and_run_filters_share_context() {
        let (_runtime, _client, history) = fixture().await;
        let filter = Filter {
            at: "example.ll:1:18".into(),
            ..Filter::default()
        };
        let selected = filter.select(&history);
        assert_eq!(
            selected
                .records
                .iter()
                .filter(|r| r.event.site.form == "(+ x x)")
                .count(),
            3
        );
        assert_eq!(
            selected
                .records
                .iter()
                .filter(|r| r.event.kind == "scope")
                .count(),
            2
        );
        let map = history
            .records
            .iter()
            .find(|r| r.event.site.form.starts_with("(map"))
            .unwrap();
        let selected = Filter {
            call: map.event.id.clone(),
            ..Filter::default()
        }
        .select(&history);
        assert_eq!(
            selected
                .records
                .iter()
                .filter(|r| r.event.kind == "callback")
                .count(),
            2
        );
        assert_eq!(
            selected
                .records
                .iter()
                .filter(|r| r.event.kind == "scope")
                .count(),
            1
        );
        let selected = Filter {
            run: map.event.run.clone(),
            ..Filter::default()
        }
        .select(&history);
        assert!(selected
            .records
            .iter()
            .all(|r| r.event.run == map.event.run));
        assert!(Filter {
            file: "not-here".into(),
            ..Filter::default()
        }
        .select(&history)
        .records
        .is_empty());
    }
    #[tokio::test]
    async fn transcripts_preserve_source_function_arguments_and_record_ids() {
        let (_runtime, _client, history) = fixture().await;
        let map = history
            .records
            .iter()
            .find(|r| r.event.site.form.starts_with("(map"))
            .unwrap();
        let mut output = Vec::new();
        print_record(&mut output, map, 240, true).unwrap();
        let output = String::from_utf8(output).unwrap();
        assert!(
            output.starts_with("(map '(2 3) twice)  # => (4 6)"),
            "{output}"
        );
        assert!(output.contains("example.ll:2:7"));
        assert!(output.contains("# arg 1: (2 3)"));
        assert!(output.contains("# arg 2: (fn (x) (+ x x))"));
        assert!(output.contains(&format!("# call {}", map.event.id)));
        let mut output = Vec::new();
        print_record(&mut output, map, 35, false).unwrap();
        assert!(String::from_utf8(output).unwrap().contains("\n# => (4 6)"));
        assert_eq!(safe("abc\u{1b}[2J"), "abc\\u{1b}[2J");
    }
    #[test]
    fn locations_reject_malformed_input_and_accept_buffer_names() {
        for bad in ["x", "x:0", "x:1:0", ":2", "x:abc"] {
            assert!(parse_location(bad).is_err(), "{bad}");
        }
        assert_eq!(
            parse_location("<buffer:notes>:8:2").unwrap(),
            ("<buffer:notes>", 8, Some(2))
        );
    }
}
