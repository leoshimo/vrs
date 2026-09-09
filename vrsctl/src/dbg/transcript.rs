//! Append-only observations. Completion sequence, not call creation order,
//! determines output order. Scope results are evaluation footers, not calls.
use super::filter::{elapsed_us, Filter, Selection};
use anyhow::Result;
use std::{collections::HashMap, io::Write};
use vrs::debug::{Record, Snapshot};

#[derive(Default)]
pub struct Transcript {
    seen: HashMap<String, u64>,
    evaluations: HashMap<String, usize>,
    pending: HashMap<String, usize>,
    next_evaluation: usize,
    next_pending: usize,
    current: Option<String>,
}
pub struct Options {
    pub all: bool,
    pub width: usize,
}

impl Transcript {
    pub fn update(
        &mut self,
        writer: &mut impl Write,
        snapshot: &Snapshot,
        filter: &Filter,
        opts: &Options,
        now: u64,
    ) -> Result<()> {
        let Selection {
            snapshot: selected,
            matches,
            ..
        } = filter.select(snapshot, now);
        let by_id: HashMap<_, _> = snapshot
            .records
            .iter()
            .map(|r| (r.event.id.as_str(), r))
            .collect();
        let mut updates: Vec<_> = selected.records.iter().collect();
        // Once a wait is announced, report its outcome even if it no longer
        // matches status::running or a duration predicate. Likewise close an
        // evaluation that the viewer already opened.
        for r in &snapshot.records {
            if (self.pending.contains_key(&r.event.id)
                || (r.event.id == r.event.run && self.evaluations.contains_key(&r.event.run)))
                && !updates.iter().any(|u| u.event.id == r.event.id)
            {
                updates.push(r);
            }
        }
        updates.sort_by_key(|r| r.sequence);
        for r in updates {
            let e = &r.event;
            let scope = e.kind == "scope" && e.id == e.run;
            let immediate = e
                .parent
                .as_deref()
                .and_then(|id| by_id.get(id))
                .is_none_or(|p| p.event.kind == "scope");
            let visible = scope
                || self.pending.contains_key(&e.id)
                || (matches.contains(&e.id) && (opts.all || immediate || filter.focused()));
            if !visible || self.seen.get(&e.id) == Some(&r.sequence) {
                continue;
            }
            if e.status == "running" && (scope || now.saturating_sub(e.started_ms) < 250) {
                continue;
            }
            let number = *self.evaluations.entry(e.run.clone()).or_insert_with(|| {
                self.next_evaluation += 1;
                self.next_evaluation
            });
            if self.current.as_ref() != Some(&e.run) {
                let origin = by_id.get(e.run.as_str()).copied().unwrap_or(r);
                writeln!(
                    writer,
                    "\n# evaluation {number} · {} · run::{}",
                    safe(&location(origin)),
                    &e.run[..e.run.len().min(8)]
                )?;
                self.current = Some(e.run.clone());
            }
            if scope {
                let result = e.result.as_ref().map(|v| v.text.as_str()).unwrap_or("");
                let status = match e.status.as_str() {
                    "returned" => "returned",
                    "error" => "failed",
                    other => other,
                };
                writeln!(
                    writer,
                    "# evaluation {number} {status}{} · {}",
                    if result.is_empty() {
                        String::new()
                    } else {
                        format!(" {}", safe(result))
                    },
                    duration(e.elapsed_us)
                )?;
            } else {
                if e.status == "running" {
                    self.pending.entry(e.id.clone()).or_insert_with(|| {
                        self.next_pending += 1;
                        self.next_pending
                    });
                }
                print_record(writer, r, opts, self.pending.get(&e.id).copied(), now)?;
            }
            self.seen.insert(e.id.clone(), r.sequence);
        }
        // Viewer state is bounded along with the daemon's ring buffer.
        self.seen.retain(|id, _| by_id.contains_key(id.as_str()));
        self.pending.retain(|id, _| by_id.contains_key(id.as_str()));
        self.evaluations
            .retain(|run, _| snapshot.records.iter().any(|r| &r.event.run == run));
        Ok(())
    }
}

pub fn location(r: &Record) -> String {
    let s = &r.event.site;
    let file = std::path::Path::new(&s.file)
        .file_name()
        .and_then(|p| p.to_str())
        .unwrap_or(&s.file);
    if s.line > 0 {
        format!("{file}:{}:{}", s.line, s.column)
    } else {
        file.into()
    }
}
pub fn duration(us: u64) -> String {
    if us >= 1_000_000 {
        format!("{:.2}s", us as f64 / 1_000_000.)
    } else {
        format!("{:.2}ms", us as f64 / 1000.)
    }
}
fn safe(text: &str) -> String {
    text.chars()
        .flat_map(|c| {
            if c.is_control() && c != '\n' {
                c.escape_default().collect::<Vec<_>>()
            } else {
                vec![c]
            }
        })
        .collect()
}
fn print_record(
    writer: &mut impl Write,
    r: &Record,
    opts: &Options,
    pending: Option<usize>,
    now: u64,
) -> Result<()> {
    let e = &r.event;
    let result = e.result.as_ref().map(|v| v.text.as_str()).unwrap_or("");
    let outcome = match e.status.as_str() {
        "running" => "running…".into(),
        "cancelled" => "cancelled".into(),
        "error" => format!("error: {result}"),
        _ => format!("=> {result}"),
    };
    let mut meta = location(r);
    if let Some(number) = pending {
        meta += &format!(" · wait {number}");
    }
    meta += &format!(" · {}", duration(elapsed_us(r, now)));
    if e.site.generated && e.kind != "callback" {
        meta += " · generated";
    }
    // A callback's source denotes the invoked function, not a function literal
    // being evaluated. Keep this boundary: it owns arguments, errors, and children.
    let invocation = if e.kind == "callback" { "invoke " } else { "" };
    let line = format!("{invocation}{}  # {outcome}  [{meta}]", e.site.form);
    if line.chars().count() <= opts.width && !line.contains('\n') {
        writeln!(writer, "{}", safe(&line))?;
    } else {
        let source = lyric::parse(&e.site.form)
            .map(|f| f.to_pretty_string(opts.width))
            .unwrap_or_else(|_| e.site.form.clone());
        writeln!(writer, "{invocation}{}  # {}", safe(&source), safe(&meta))?;
        for line in safe(&outcome).lines() {
            writeln!(writer, "# {line}")?;
        }
    }
    writeln!(writer, "  # call::{}", &e.id[..e.id.len().min(8)])?;
    for (i, arg) in e.arguments.iter().enumerate() {
        let value = format!(
            "arg {}: {}{}",
            i + 1,
            safe(&arg.text),
            if arg.truncated { " [truncated]" } else { "" }
        );
        for line in value.lines() {
            writeln!(writer, "  # {line}")?;
        }
    }
    if e.arguments_truncated {
        writeln!(writer, "  # additional arguments omitted")?;
    }
    if e.result.as_ref().is_some_and(|r| r.truncated) {
        writeln!(writer, "  # result truncated")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dbg::tests::fixture;
    #[tokio::test]
    async fn completions_precede_scope_footer_and_filtering_a_file_does_not_expand() {
        let (_runtime, _client, history) = fixture().await;
        let mut out = vec![];
        let opts = Options {
            all: false,
            width: 120,
        };
        Transcript::default()
            .update(
                &mut out,
                &history,
                &Filter::parse("file::example.ll").unwrap(),
                &opts,
                u64::MAX,
            )
            .unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(
            text.find("(map '(2 3) twice)").unwrap() < text.find("evaluation 1 returned").unwrap(),
            "{text}"
        );
        assert!(!text.contains("(dbg!"));
        assert!(!text.lines().any(|line| line.starts_with("(+ x x)")));
        assert!(text.contains("  # arg 1: (2 3)"));
        assert!(text.contains("  # arg 1: 4"));
        assert!(text.contains("  # arg 2: (fn (x) (+ x x))"));
        assert!(text.contains("ms"));
    }
    #[tokio::test]
    async fn pending_gets_elapsed_time_and_same_label_on_completion_without_duplicates() {
        let (_runtime, _client, mut history) = fixture().await;
        history.records.retain(|r| r.event.site.form == "(twice 4)");
        let returned = history.clone();
        history.records[0].event.status = "running".into();
        history.records[0].event.result = None;
        history.records[0].sequence = 1;
        let now = history.records[0].event.started_ms + 1250;
        let opts = Options {
            all: true,
            width: 160,
        };
        let mut transcript = Transcript::default();
        let mut out = vec![];
        transcript
            .update(
                &mut out,
                &history,
                &Filter::parse("status::running").unwrap(),
                &opts,
                now,
            )
            .unwrap();
        transcript
            .update(
                &mut out,
                &returned,
                &Filter::parse("status::running").unwrap(),
                &opts,
                now,
            )
            .unwrap();
        let size = out.len();
        transcript
            .update(
                &mut out,
                &returned,
                &Filter::parse("status::running").unwrap(),
                &opts,
                now,
            )
            .unwrap();
        assert_eq!(size, out.len());
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains("# running…"));
        assert!(text.contains("1.25s"));
        assert_eq!(text.matches("wait 1").count(), 2);
        assert!(text.contains("# => 8"));
        assert_eq!(safe("\u{1b}[2J"), "\\u{1b}[2J");
    }
}
