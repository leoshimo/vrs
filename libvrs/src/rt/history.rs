//! Recent submitted programs and handled service calls, owned by the runtime.
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};

use lyric::{Error, Form, Result};

use super::program::{Extern, Fiber, NativeAsyncFn, NativeFn, NativeFnOp, Val};
use super::ProcessId;

const ENTRIES_PER_PROCESS: usize = 20;
const BYTES_PER_PROCESS: usize = 256 * 1024;
const TOTAL_BYTES: usize = 4 * 1024 * 1024;
const MAX_PROCESSES: usize = 512;
const MAX_DEPTH: usize = 128;

#[derive(Debug, Default)]
struct History {
    entries: VecDeque<(Form, usize)>,
    bytes: usize,
    last_append: u64,
}

#[derive(Debug, Default)]
struct Histories {
    processes: HashMap<ProcessId, History>,
    bytes: usize,
    sequence: u64,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct Store(Arc<Mutex<Histories>>);

impl PartialEq for Store {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Store {
    pub(crate) fn append(&self, pid: &ProcessId, value: &Val) {
        // History must not retain captured environments or source-provenance
        // graphs. Oversized or opaque values are omitted, never stringified
        // into something that could be mistaken for executable source.
        let mut remaining = BYTES_PER_PROCESS;
        let Some(form) = snapshot(value, &mut remaining, 0) else {
            return;
        };
        let bytes = BYTES_PER_PROCESS - remaining;
        let mut histories = self.0.lock().unwrap();
        histories.sequence += 1;
        let sequence = histories.sequence;
        let history = histories.processes.entry(pid.clone()).or_default();
        let previous_bytes = history.bytes;
        history.entries.push_back((form, bytes));
        history.bytes += bytes;
        history.last_append = sequence;
        while history.entries.len() > ENTRIES_PER_PROCESS || history.bytes > BYTES_PER_PROCESS {
            let (_, removed) = history.entries.pop_front().unwrap();
            history.bytes -= removed;
        }
        let current_bytes = history.bytes;
        histories.bytes = histories.bytes - previous_bytes + current_bytes;
        // Also bound histories of processes that have exited. Evict the least
        // recently written process history when the runtime budget is full.
        while histories.bytes > TOTAL_BYTES || histories.processes.len() > MAX_PROCESSES {
            let oldest = histories
                .processes
                .iter()
                .min_by_key(|(_, history)| history.last_append)
                .map(|(pid, _)| pid.clone())
                .unwrap();
            histories.bytes -= histories.processes.remove(&oldest).unwrap().bytes;
        }
    }

    pub(crate) fn read(&self, pid: &ProcessId) -> Val {
        let histories = self.0.lock().unwrap();
        Val::List(
            histories
                .processes
                .get(pid)
                .into_iter()
                .flat_map(|history| &history.entries)
                .map(|(form, _)| Val::from(form.clone()))
                .collect(),
        )
    }
}

fn snapshot(value: &Val, remaining: &mut usize, depth: usize) -> Option<Form> {
    if depth > MAX_DEPTH {
        return None;
    }
    *remaining = remaining.checked_sub(std::mem::size_of::<Form>())?;
    Some(match value {
        Val::Nil => Form::Nil,
        Val::Bool(value) => Form::Bool(*value),
        Val::Int(value) => Form::Int(*value),
        Val::String(value) => {
            *remaining = remaining.checked_sub(value.len())?;
            Form::String(value.clone())
        }
        Val::Symbol(value) => {
            *remaining = remaining.checked_sub(value.as_str().len())?;
            Form::symbol(value.as_str())
        }
        Val::Keyword(value) => {
            *remaining = remaining.checked_sub(value.as_str().len())?;
            Form::keyword(value.as_str())
        }
        Val::List(values) => Form::List(
            values
                .iter()
                .map(|value| snapshot(value, remaining, depth + 1))
                .collect::<Option<Vec<_>>>()?,
        ),
        _ => return None,
    })
}

pub(crate) fn append_fn() -> NativeFn {
    NativeFn {
        metadata: vec![],
        doc: "(history_append EXPRESSION) - For runtime internal use: append an expression to this process's bounded history without evaluating it. Oversized or non-source values are omitted.".into(),
        func: |fiber, args| {
            let [expression] = args else {
                return Err(Error::UnexpectedArguments("history_append expects one expression".into()));
            };
            if let Some(store) = &fiber.locals().history {
                store.append(&fiber.locals().pid, expression);
            }
            Ok(NativeFnOp::Return(Val::keyword("ok")))
        },
    }
}

pub(crate) fn history_fn() -> NativeAsyncFn {
    NativeAsyncFn {
        metadata: vec![],
        doc: "(history [PID-OR-SERVICE]) - Return recent history entries, oldest first. Defaults to this process; a service name selects its current instance.".into(),
        func: |fiber, args| Box::new(history_impl(fiber, args)),
    }
}

async fn history_impl(fiber: &mut Fiber, args: Vec<Val>) -> Result<Val> {
    let pid = match args.as_slice() {
        [] => fiber.locals().pid.clone(),
        [Val::Extern(Extern::ProcessId(pid))] => pid.clone(),
        [Val::Keyword(name)] => fiber
            .locals()
            .registry
            .as_ref()
            .ok_or_else(|| Error::Runtime("no registry".into()))?
            .lookup(name.clone())
            .await
            .map_err(|error| Error::Runtime(error.to_string()))?
            .ok_or_else(|| Error::Runtime(format!("No service found for {name}")))?
            .pid(),
        _ => {
            return Err(Error::UnexpectedArguments(
                "history expects an optional PID or service name".into(),
            ))
        }
    };
    if pid.node() != fiber.locals().node_name {
        // Query the owning runtime, not the target process: busy and departed
        // remote processes have the same history behavior as local processes.
        let code = Val::List(vec![
            Val::symbol("history"),
            Val::Extern(Extern::ProcessId(pid.clone())),
        ]);
        return fiber
            .locals()
            .peers
            .as_ref()
            .ok_or_else(|| Error::Runtime("no node transport".into()))?
            .eval(pid.node().to_string(), code)
            .await
            .map_err(|error| match error {
                crate::Error::EvaluationError(error) => error,
                error => Error::Runtime(error.to_string()),
            });
    }
    Ok(fiber
        .locals()
        .history
        .as_ref()
        .map_or_else(|| Val::List(vec![]), |store| store.read(&pid)))
}

pub(crate) fn submission_fn() -> NativeFn {
    NativeFn {
        metadata: vec![],
        doc: "For runtime internal use: extract submitted code from an eval_source request without executing it.".into(),
        func: |_, args| {
            let [request] = args else {
                return Err(Error::UnexpectedArguments("vrs/history_submission expects one request".into()));
            };
            // TODO: carry source locations in the request rather than an
            // executable wrapper. Keep evaluating the original request.
            if let Val::List(items) = request {
                if let [Val::Symbol(name), Val::String(source), _, _, _] = items.as_slice() {
                    if name.as_str() == "eval_source" {
                        if let Ok(mut forms) = lyric::parse_script(source) {
                            let form = if forms.len() == 1 {
                                forms.remove(0)
                            } else {
                                Form::List(std::iter::once(Form::symbol("begin")).chain(forms).collect())
                            };
                            return Ok(NativeFnOp::Return(Val::from(form)));
                        }
                    }
                }
            }
            // Malformed source is still a submission. Preserve its request;
            // evaluation, not history extraction, reports the source error.
            Ok(NativeFnOp::Return(request.clone()))
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retention_is_bounded_by_count_and_bytes() {
        let store = Store::default();
        let pid = ProcessId::new("test", 1);
        for n in 0..25 {
            store.append(&pid, &Val::Int(n));
        }
        assert_eq!(store.read(&pid), Val::List((5..25).map(Val::Int).collect()));

        let first = Val::String("x".repeat(BYTES_PER_PROCESS / 2));
        let second = Val::String("y".repeat(BYTES_PER_PROCESS / 2));
        store.append(&pid, &first);
        store.append(&pid, &second);
        assert_eq!(store.read(&pid), Val::List(vec![second]));
        assert!(store.0.lock().unwrap().bytes <= BYTES_PER_PROCESS);
    }

    #[test]
    fn process_histories_and_total_retained_memory_are_bounded() {
        let store = Store::default();
        for n in 0..=MAX_PROCESSES {
            store.append(&ProcessId::new("test", n), &Val::Int(1));
        }
        assert_eq!(store.0.lock().unwrap().processes.len(), MAX_PROCESSES);
        assert_eq!(store.read(&ProcessId::new("test", 0)), Val::List(vec![]));

        let large = Val::String("x".repeat(BYTES_PER_PROCESS / 2));
        for n in 0..100 {
            store.append(&ProcessId::new("large", n), &large);
        }
        let histories = store.0.lock().unwrap();
        assert!(histories.bytes <= TOTAL_BYTES);
        assert_eq!(
            histories.bytes,
            histories
                .processes
                .values()
                .map(|history| history.bytes)
                .sum::<usize>()
        );
    }

    #[test]
    fn unrepresentable_oversized_and_deep_entries_do_not_replace_good_history() {
        let store = Store::default();
        let pid = ProcessId::new("test", 1);
        store.append(&pid, &Val::Int(7));
        store.append(&pid, &Val::String("x".repeat(BYTES_PER_PROCESS)));
        store.append(&pid, &Val::Extern(Extern::ProcessId(pid.clone())));
        let mut deep = Val::Int(0);
        for _ in 0..=MAX_DEPTH {
            deep = Val::List(vec![deep]);
        }
        store.append(&pid, &deep);
        assert_eq!(store.read(&pid), Val::List(vec![Val::Int(7)]));
    }

    #[test]
    fn stored_syntax_does_not_keep_source_provenance_alive() {
        let forms = lyric::parse_source("(+ 1 2)", "a-very-large-buffer.ll", 10, 1).unwrap();
        let value = Val::from(forms[0].clone());
        assert!(lyric::source::site(&value).is_some());
        let store = Store::default();
        let pid = ProcessId::new("test", 1);
        store.append(&pid, &value);
        let Val::List(entries) = store.read(&pid) else {
            unreachable!()
        };
        assert_eq!(entries, vec![value]);
        assert!(lyric::source::site(&entries[0]).is_none());
    }
}
