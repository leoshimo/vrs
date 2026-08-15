//! Source-embedded observations: one filter language, streaming CLI, web inspector.
mod filter;
mod transcript;
mod web;
use anyhow::{Context, Result};
use clap::{Arg, ArgAction, ArgMatches, Command};
use filter::{now_ms, Filter};
use std::{
    collections::HashMap,
    io::{self, Write},
    time::Duration,
};
use vrs::{debug::Snapshot, Client};

pub fn command() -> Command {
    Command::new("dbg")
        .about("Follow dbg! calls with arguments, results, and elapsed time")
        .after_help(filter::HELP)
        .arg(
            Arg::new("web")
                .long("web")
                .action(ArgAction::SetTrue)
                .help("Open the browser call inspector"),
        )
        .arg(
            Arg::new("filter")
                .long("filter")
                .value_name("QUERY")
                .default_value("")
                .help("One query, e.g. 'file:scratch.ll:7 expr:sleep' (see below)"),
        )
        .arg(
            Arg::new("all")
                .long("all")
                .action(ArgAction::SetTrue)
                .help("Include nested calls"),
        )
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
    let filter = Filter::parse(args.get_one::<String>("filter").unwrap())?;
    if args.get_flag("web") {
        return web::run(client, filter, args.get_flag("all")).await;
    }
    let opts = transcript::Options {
        all: args.get_flag("all"),
        width,
    };
    let mut interrupt = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt())?;
    let mut subscription = client.subscribe(vrs::debug::TOPIC.into()).await?;
    let initial = snapshot(client).await?;
    let mut transcript = transcript::Transcript::after(initial.cursor);
    let mut dropped = initial.dropped;
    loop {
        let history = snapshot(client).await?;
        if history.dropped > dropped {
            eprintln!("dbg: {} observations dropped", history.dropped - dropped);
        }
        dropped = history.dropped;
        transcript.update(&mut io::stdout(), &history, &filter, &opts, now_ms())?;
        io::stdout().flush()?;
        tokio::select! {
            _=interrupt.recv()=>return Ok(()),
            event=subscription.recv()=>{ event.context("Debug subscription closed")?; },
            _=tokio::time::sleep(Duration::from_millis(250))=>(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vrs::{Connection, Runtime};
    pub async fn fixture() -> (Runtime, Client, Snapshot) {
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
        let history = snapshot(&client).await.unwrap();
        (runtime, client, history)
    }
    #[tokio::test]
    async fn shared_filters_find_nested_calls_and_preserve_context() {
        let (_runtime, _client, history) = fixture().await;
        let selected = Filter::parse("file::example.ll:1:18 expr::\"(+ x x)\"")
            .unwrap()
            .select(&history, now_ms());
        assert_eq!(selected.matches.len(), 3);
        assert_eq!(
            selected
                .snapshot
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
        let selected = Filter::parse(&format!("call::{}", map.event.id))
            .unwrap()
            .select(&history, now_ms());
        assert_eq!(
            selected
                .snapshot
                .records
                .iter()
                .filter(|r| r.event.kind == "callback")
                .count(),
            2
        );
        let selected = Filter::parse(&format!("run::{}", map.event.run))
            .unwrap()
            .select(&history, now_ms());
        assert!(selected
            .snapshot
            .records
            .iter()
            .all(|r| r.event.run == map.event.run));
        assert!(Filter::parse("file::not-here")
            .unwrap()
            .select(&history, now_ms())
            .matches
            .is_empty());
        assert!(Filter::parse("status::error")
            .unwrap()
            .select(&history, now_ms())
            .matches
            .is_empty());
    }
    #[test]
    fn help_exposes_only_the_simplified_interface() {
        command().debug_assert();
        for removed in [
            "--once",
            "--json",
            "--no-open",
            "--details",
            "--file",
            "--at",
            "--expr",
            "--run",
            "--call",
            "--values",
            "--time",
        ] {
            assert!(
                command().try_get_matches_from(["dbg", removed]).is_err(),
                "{removed}"
            );
        }
        assert!(command()
            .try_get_matches_from(["dbg", "--filter", "file::x:1", "--all"])
            .is_ok());
    }

    #[tokio::test]
    async fn duration_filters_measure_running_calls_and_combine_with_status() {
        let (_runtime, _client, mut history) = fixture().await;
        history.records.truncate(1);
        let r = &mut history.records[0];
        r.event.status = "running".into();
        r.event.elapsed_us = 0;
        let now = r.event.started_ms + 250;
        let filter = Filter::parse("time::>100ms status::running").unwrap();
        assert_eq!(filter.select(&history, now).matches.len(), 1);
        history.records[0].event.status = "returned".into();
        history.records[0].event.elapsed_us = 50_000;
        assert!(filter.select(&history, now).matches.is_empty());
        assert_eq!(
            Filter::parse("time::<0.1s status::returned")
                .unwrap()
                .select(&history, now)
                .matches
                .len(),
            1
        );
    }
}
