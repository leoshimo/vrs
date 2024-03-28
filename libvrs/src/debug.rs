//! One bounded observation history per runtime, exposed as ordinary Lyric data.
//! Live updates use the existing :dbg pub/sub topic. Snapshots repair missed
//! notifications and let viewers attach after an evaluation has completed.
use crate::rt::pubsub::PubSubHandle;
use crate::{Form, NativeFn, NativeFnOp, Val};
use lyric::debug::{Event, Observer};
use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
};

pub const TOPIC: &str = "dbg";
pub const CAPACITY: usize = 512;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Record {
    pub sequence: u64,
    pub process: String,
    #[serde(flatten)]
    pub event: Event,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub cursor: u64,
    pub dropped: u64,
    pub evicted: u64,
    pub records: Vec<Record>,
}
impl Snapshot {
    pub fn from_form(form: Form) -> Result<Self, serde_json::Error> {
        serde_json::from_value(from_form(form))
    }
}
#[derive(Debug, Default)]
struct History {
    cursor: u64,
    evicted: u64,
    records: VecDeque<Record>,
}
#[derive(Debug)]
struct Inner {
    history: Mutex<History>,
    dropped: AtomicU64,
    pubsub: PubSubHandle,
}
#[derive(Debug, Clone)]
pub(crate) struct Store(Arc<Inner>);
impl PartialEq for Store {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Store {
    pub(crate) fn new(pubsub: PubSubHandle) -> Self {
        Self(Arc::new(Inner {
            history: Mutex::new(History::default()),
            dropped: AtomicU64::new(0),
            pubsub,
        }))
    }
    pub(crate) fn observer(&self, process: String) -> Arc<dyn Observer> {
        Arc::new(ProcessObserver {
            store: self.clone(),
            process,
        })
    }
    pub(crate) fn snapshot(&self) -> Snapshot {
        let history = self.0.history.lock().unwrap();
        Snapshot {
            cursor: history.cursor,
            evicted: history.evicted,
            dropped: self.0.dropped.load(Ordering::Relaxed),
            records: history.records.iter().cloned().collect(),
        }
    }
}
#[derive(Debug)]
struct ProcessObserver {
    store: Store,
    process: String,
}
impl Observer for ProcessObserver {
    fn observe(&self, event: Event) {
        let Ok(mut history) = self.store.0.history.try_lock() else {
            self.store.0.dropped.fetch_add(1, Ordering::Relaxed);
            return;
        };
        history.cursor += 1;
        let record = Record {
            sequence: history.cursor,
            process: self.process.clone(),
            event,
        };
        if let Some(old) = history
            .records
            .iter_mut()
            .find(|old| old.event.id == record.event.id)
        {
            *old = record.clone();
        } else {
            if history.records.len() == CAPACITY {
                history.records.pop_front();
                history.evicted += 1;
            }
            history.records.push_back(record.clone());
        }
        drop(history);
        // A full publication queue only loses a notification: the snapshot
        // still contains the record. Viewers reconcile with that snapshot.
        let form = to_form(serde_json::to_value(&record).expect("debug record serializes"));
        let _ = self
            .store
            .0
            .pubsub
            .try_publish(&TOPIC.into(), Val::from(form));
        if record.event.status != "running" {
            tracing::info!(target:"vrs::dbg", process=%record.process, call=%record.event.id,
                "{}:{}:{} {} => {} [{}]", record.event.site.file, record.event.site.line,
                record.event.site.column, record.event.site.form,
                record.event.result.as_ref().map(|r| r.text.as_str()).unwrap_or(""), record.event.status);
        }
    }
}

pub(crate) fn history_fn() -> NativeFn {
    NativeFn {
        metadata: vec![],
        doc: "(dbg_history) - Return the bounded shared dbg! recording, cursor, and loss counters."
            .into(),
        func: |fiber, args| {
            if !args.is_empty() {
                return Err(lyric::Error::UnexpectedArguments(
                    "dbg_history expects no arguments".into(),
                ));
            }
            let store = fiber
                .locals()
                .debug
                .as_ref()
                .ok_or_else(|| lyric::Error::Runtime("no debug recorder attached".into()))?;
            let snapshot = store.snapshot();
            Ok(NativeFnOp::Return(Val::from(to_form(
                serde_json::to_value(snapshot).unwrap(),
            ))))
        },
    }
}

// Lyric has 32-bit integers; larger counters/timestamps travel as decimal
// strings. Everything else is the usual plist/list/scalar representation.
fn to_form(value: serde_json::Value) -> Form {
    use serde_json::Value::*;
    match value {
        Null => Form::Nil,
        Bool(v) => Form::Bool(v),
        String(v) => Form::String(v),
        Number(v) => v
            .as_i64()
            .and_then(|n| i32::try_from(n).ok())
            .map(Form::Int)
            .unwrap_or_else(|| Form::String(v.to_string())),
        Array(items) => Form::List(items.into_iter().map(to_form).collect()),
        Object(items) => Form::List(
            items
                .into_iter()
                .flat_map(|(k, v)| [Form::keyword(&k), to_form(v)])
                .collect(),
        ),
    }
}
fn from_form(form: Form) -> serde_json::Value {
    use serde_json::{Map, Value};
    match form {
        Form::Nil => Value::Null,
        Form::Bool(v) => Value::Bool(v),
        Form::Int(v) => v.into(),
        Form::String(v) | Form::RawString(v) => Value::String(v),
        Form::Keyword(v) => Value::String(v.to_string()),
        Form::Symbol(v) => Value::String(v.to_string()),
        Form::List(items)
            if matches!(items.first(), Some(Form::Keyword(_))) && items.len() % 2 == 0 =>
        {
            let mut map = Map::new();
            let mut items = items.into_iter();
            while let (Some(Form::Keyword(k)), Some(v)) = (items.next(), items.next()) {
                let key = k.as_str().to_owned();
                let mut value = from_form(v);
                if matches!(
                    key.as_str(),
                    "cursor" | "sequence" | "started_ms" | "elapsed_us" | "dropped" | "evicted"
                ) {
                    if let Some(n) = value.as_str().and_then(|s| s.parse::<u64>().ok()) {
                        value = n.into();
                    }
                }
                map.insert(key, value);
            }
            Value::Object(map)
        }
        Form::List(items) => Value::Array(items.into_iter().map(from_form).collect()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Client, Connection, Runtime};
    use std::time::Duration;
    async fn connect(runtime: &Runtime) -> Client {
        let (local, remote) = Connection::pair().unwrap();
        runtime.handle_conn(remote).await.unwrap();
        Client::new(local)
    }
    async fn snapshot(client: &Client) -> Snapshot {
        Snapshot::from_form(
            client
                .request(lyric::parse("(dbg_history)").unwrap())
                .await
                .unwrap()
                .contents
                .unwrap(),
        )
        .unwrap()
    }
    #[tokio::test]
    async fn shared_history_and_existing_pubsub_include_async_completion_and_source() {
        let runtime = Runtime::new("debug-test");
        let worker = connect(&runtime).await;
        let viewer = connect(&runtime).await;
        let mut sub = viewer.subscribe(TOPIC.into()).await.unwrap();
        // Establish ordering across the client's subscription/evaluation messages.
        snapshot(&viewer).await;
        let value = worker
            .request(lyric::source::request(
                "(dbg! (sleep 1) (+ 20 22))",
                "worker.ll",
                10,
                1,
            ))
            .await
            .unwrap()
            .contents
            .unwrap();
        assert_eq!(value, Form::Int(42));
        let update = tokio::time::timeout(Duration::from_secs(2), sub.recv())
            .await
            .unwrap()
            .unwrap();
        assert!(matches!(update, Form::List(_)));
        let late = connect(&runtime).await;
        let history = snapshot(&late).await;
        assert_eq!(history.records.len(), 3);
        assert!(history.records.iter().all(|r| r.event.status == "returned"));
        let sleep = history
            .records
            .iter()
            .find(|r| r.event.site.form == "(sleep 1)")
            .unwrap();
        assert_eq!(sleep.event.site.file, "worker.ll");
        assert_eq!(sleep.event.site.line, 10);
        assert!(sleep.event.elapsed_us >= 500);
        assert!(sleep.process.contains("debug-test"));
    }
    #[tokio::test]
    async fn disconnect_cancels_inflight_scope_and_retains_completed_values() {
        let runtime = Runtime::new("cancel-test");
        let worker = connect(&runtime).await;
        let viewer = connect(&runtime).await;
        let request = worker.request(lyric::source::request(
            "(dbg! (+ 1 2) (sleep 10000) (error \"must not run\"))",
            "cancel.ll",
            1,
            1,
        ));
        tokio::pin!(request);
        tokio::select! { _ = &mut request => panic!("must still be sleeping"), _=tokio::time::sleep(Duration::from_millis(30))=>() }
        worker.shutdown().await;
        let history = tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                let history = snapshot(&viewer).await;
                if history
                    .records
                    .iter()
                    .any(|r| r.event.status == "cancelled")
                {
                    break history;
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        assert_eq!(history.records.len(), 3);
        assert!(history
            .records
            .iter()
            .any(|r| r.event.site.form == "(+ 1 2)" && r.event.status == "returned"));
        assert_eq!(
            history
                .records
                .iter()
                .filter(|r| r.event.status == "cancelled")
                .count(),
            2
        );
    }
    #[tokio::test]
    async fn bounded_history_and_loss_are_explicit() {
        let store = Store::new(crate::rt::pubsub::PubSub::spawn());
        let observer = store.observer("test".into());
        for i in 0..CAPACITY + 3 {
            observer.observe(Event {
                id: i.to_string(),
                parent: None,
                run: i.to_string(),
                kind: "scope".into(),
                site: lyric::source::SourceSite::synthetic("(dbg! 1)".into()),
                arguments: vec![],
                arguments_truncated: false,
                result: None,
                status: "running".into(),
                elapsed_us: 0,
                started_ms: 0,
            });
        }
        let s = store.snapshot();
        assert_eq!(s.records.len(), CAPACITY);
        assert_eq!(s.evicted, 3);
        let _held = store.0.history.lock().unwrap();
        observer.observe(s.records[0].event.clone());
        assert_eq!(store.0.dropped.load(Ordering::Relaxed), 1);
    }
}
