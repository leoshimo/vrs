//! A single query language, shared by the terminal and browser.
use anyhow::{bail, ensure, Context, Result};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use vrs::debug::{Record, Snapshot};

pub const HELP: &str = "Filter syntax (terms are ANDed; double-quote text containing spaces):\n  file::scratch.ll       file path contains scratch.ll\n  file::scratch.ll:7:3   exact line and optional column\n  expr::\"(+ x x)\"       source expression contains (+ x x)\n  status::error          running, returned, error, or cancelled\n  time::>100ms           elapsed wall time, including waits and children\n  run::ID / call::ID     one evaluation / one call and its descendants\nPlain words search source expressions. Click locations in the browser to add filters.\nHistory is memory-only: at most 512 records and 4 MiB of encoded data per runtime.\nCtrl-C closes this viewer; it does not cancel the observed evaluation.";

#[derive(Debug, Default, Clone)]
pub struct Filter {
    pub query: String,
    terms: Vec<Term>,
}
#[derive(Debug, Clone)]
enum Term {
    File(String, Option<(usize, Option<usize>)>),
    Expr(String),
    Status(String),
    Time(bool, f64),
    Run(String),
    Call(String),
}
#[derive(Serialize)]
pub struct Selection {
    #[serde(flatten)]
    pub snapshot: Snapshot,
    /// Ancestors provide inspection context, but do not automatically expand.
    pub matches: HashSet<String>,
    pub focused: bool,
}

impl Filter {
    pub fn parse(query: &str) -> Result<Self> {
        let mut terms = Vec::new();
        for token in tokenize(query)? {
            let (key, value) = token.split_once("::").unwrap_or(("expr", &token));
            ensure!(!value.is_empty(), "{key} requires a value");
            terms.push(match key {
                "file" => {
                    let (file, location) = file_location(value)?;
                    Term::File(file.into(), location)
                }
                "expr" => Term::Expr(value.into()),
                "run" => Term::Run(value.into()),
                "call" => Term::Call(value.into()),
                "status" => {
                    ensure!(matches!(value, "running" | "returned" | "error" | "cancelled"),
                        "status must be running, returned, error, or cancelled");
                    Term::Status(value.into())
                }
                "time" => {
                    let (greater, duration) = if let Some(v) = value.strip_prefix('>') {
                        (true, v)
                    } else if let Some(v) = value.strip_prefix('<') {
                        (false, v)
                    } else { bail!("time expects >100ms or <1s"); };
                    let (number, scale) = if let Some(v) = duration.strip_suffix("ms") {
                        (v, 1000.)
                    } else if let Some(v) = duration.strip_suffix('s') {
                        (v, 1_000_000.)
                    } else { bail!("time requires ms or s units"); };
                    let micros = number.parse::<f64>().context("invalid duration")? * scale;
                    ensure!(micros.is_finite() && micros >= 0., "invalid duration");
                    Term::Time(greater, micros)
                }
                _ => bail!("Unknown filter {key:?}; use file::, expr::, status::, time::, run::, or call::"),
            });
        }
        Ok(Self {
            query: query.into(),
            terms,
        })
    }
    /// A file/run restriction preserves the default overview. A focused query
    /// can find a particular invocation even when it is nested.
    pub fn focused(&self) -> bool {
        self.terms
            .iter()
            .any(|t| !matches!(t, Term::File(_, None) | Term::Run(_)))
    }

    pub fn select(&self, snapshot: &Snapshot, now_ms: u64) -> Selection {
        let records: HashMap<_, _> = snapshot
            .records
            .iter()
            .map(|r| (r.event.id.as_str(), r))
            .collect();
        let mut matches = HashSet::new();
        let mut selected = HashSet::new();
        for r in &snapshot.records {
            let e = &r.event;
            let matched = self.terms.iter().all(|term| match term {
                Term::Expr(text) => e.site.form.contains(text),
                Term::Status(status) => &e.status == status,
                Term::Run(id) => e.run.starts_with(id),
                Term::File(file, location) => match location {
                    None => e.site.file.contains(file),
                    Some((line, column)) => {
                        (e.site.file == *file || e.site.file.ends_with(&format!("/{file}")))
                            && e.site.line == *line
                            && column.is_none_or(|c| c == e.site.column)
                    }
                },
                Term::Time(greater, threshold) => {
                    let elapsed = elapsed_us(r, now_ms) as f64;
                    if *greater {
                        elapsed > *threshold
                    } else {
                        elapsed < *threshold
                    }
                }
                Term::Call(id) => {
                    let mut current = Some(r);
                    let mut found = false;
                    for _ in 0..=records.len() {
                        let Some(r) = current else {
                            break;
                        };
                        if r.event.id.starts_with(id) {
                            found = true;
                            break;
                        }
                        current = r
                            .event
                            .parent
                            .as_deref()
                            .and_then(|p| records.get(p).copied());
                    }
                    found
                }
            });
            if !matched {
                continue;
            }
            matches.insert(e.id.clone());
            selected.insert(e.id.as_str());
            let mut parent = e.parent.as_deref();
            for _ in 0..records.len() {
                let Some(id) = parent else {
                    break;
                };
                if !selected.insert(id) {
                    break;
                }
                parent = records.get(id).and_then(|r| r.event.parent.as_deref());
            }
        }
        Selection {
            snapshot: Snapshot {
                records: snapshot
                    .records
                    .iter()
                    .filter(|r| selected.contains(r.event.id.as_str()))
                    .cloned()
                    .collect(),
                ..snapshot.clone()
            },
            matches,
            focused: self.focused(),
        }
    }
}

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
pub fn elapsed_us(record: &Record, now_ms: u64) -> u64 {
    if record.event.status == "running" {
        now_ms
            .saturating_sub(record.event.started_ms)
            .saturating_mul(1000)
    } else {
        record.event.elapsed_us
    }
}

fn tokenize(query: &str) -> Result<Vec<String>> {
    let mut result = Vec::new();
    let mut token = String::new();
    let mut quoted = false;
    let mut chars = query.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '"' => quoted = !quoted,
            '\\' if quoted && matches!(chars.peek(), Some('"' | '\\')) => {
                token.push(chars.next().unwrap())
            }
            ch if ch.is_whitespace() && !quoted => {
                if !token.is_empty() {
                    result.push(std::mem::take(&mut token));
                }
            }
            ch => token.push(ch),
        }
    }
    ensure!(!quoted, "Unclosed double quote in filter");
    if !token.is_empty() {
        result.push(token);
    }
    Ok(result)
}
type Location<'a> = (&'a str, Option<(usize, Option<usize>)>);
fn file_location(value: &str) -> Result<Location<'_>> {
    let Some((before, last)) = value.rsplit_once(':') else {
        return Ok((value, None));
    };
    // Colons within named editor buffers are ordinary filename characters.
    if !last.chars().all(|c| c.is_ascii_digit()) {
        return Ok((value, None));
    }
    let last: usize = last
        .parse()
        .context("file location requires a positive line/column")?;
    ensure!(
        last > 0 && !before.is_empty(),
        "file location requires a filename and positive line/column"
    );
    if let Some((file, line)) = before.rsplit_once(':') {
        if line.chars().all(|c| c.is_ascii_digit()) {
            let line: usize = line.parse().context("invalid source line")?;
            ensure!(line > 0 && !file.is_empty(), "invalid source location");
            return Ok((file, Some((line, Some(last)))));
        }
    }
    Ok((before, Some((last, None))))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn query_handles_quoted_source_paths_buffers_and_invalid_terms() {
        assert_eq!(
            tokenize("file::\"my file.ll:7:3\" expr::\"(map '(2 3) twice)\"").unwrap(),
            ["file::my file.ll:7:3", "expr::(map '(2 3) twice)"]
        );
        assert_eq!(
            file_location("<buffer:notes>:8:2").unwrap(),
            ("<buffer:notes>", Some((8, Some(2))))
        );
        for query in [
            "file::x:0",
            "file::x:1:0",
            "file::",
            "wat::x",
            "expr::\"oops",
            "time::>NaNms",
            "time::100ms",
            "status::pending",
        ] {
            assert!(Filter::parse(query).is_err(), "{query}");
        }
    }
}
