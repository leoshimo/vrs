//! One bounded observation history per runtime, exposed as ordinary Lyric data.
//! Live updates use the existing :dbg pub/sub topic. Snapshots repair missed
//! notifications and let viewers attach after an evaluation has completed.
use crate::rt::pubsub::PubSubHandle;
use crate::{Form, NativeFn, NativeFnOp, Val};
use lyric::debug::{Event, Observer};
use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

pub const TOPIC: &str = "dbg";
pub const CAPACITY: usize = 512;
// Leave ample room beneath the connection codec's 8 MiB frame limit for
// snapshot fields and the response envelope, including JSON escaping.
const WIRE_BUDGET: usize = 4 * 1024 * 1024;

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
#[derive(Debug)]
struct History {
    cursor: u64,
    evicted: u64,
    records: VecDeque<(Arc<Record>, usize)>,
    wire_bytes: usize,
}
impl Default for History {
    fn default() -> Self {
        Self {
            cursor: 0,
            evicted: 0,
            records: VecDeque::with_capacity(CAPACITY + 1),
            wire_bytes: 0,
        }
    }
}
#[derive(Debug)]
struct Inner {
    history: Mutex<History>,
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
        // Only copy bounded record handles while synchronized. Cloning values,
        // serialization, and viewer I/O happen after releasing the lock.
        let mut records = Vec::with_capacity(CAPACITY);
        let (cursor, evicted) = {
            let history = self.0.history.lock().unwrap();
            records.extend(history.records.iter().map(|(r, _)| Arc::clone(r)));
            (history.cursor, history.evicted)
        };
        Snapshot {
            cursor,
            evicted,
            // Kept in the wire format for compatibility with existing viewers.
            dropped: 0,
            records: records
                .into_iter()
                .map(|record| (*record).clone())
                .collect(),
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
        let record = Record {
            sequence: 0,
            process: self.process.clone(),
            event,
        };
        let mut form = to_form(serde_json::to_value(&record).expect("debug record serializes"));
        // Measure the actual Form encoding, rather than the smaller viewer JSON.
        // The margin covers replacing sequence=0 with any 64-bit decimal value.
        let wire_bytes = serde_json::to_vec(&form)
            .expect("debug form serializes")
            .len()
            + 32;
        let mut record = Arc::new(record);
        // Retire payloads outside the critical section too. A tiny index update
        // is serialized, rather than losing completions when a viewer reads.
        let mut retired = Vec::with_capacity(CAPACITY + 1);
        let mut history = self.store.0.history.lock().unwrap();
        history.cursor += 1;
        Arc::get_mut(&mut record).unwrap().sequence = history.cursor;
        if let Some(index) = history
            .records
            .iter()
            .position(|(old, _)| old.event.id == record.event.id)
        {
            history.wire_bytes -= history.records[index].1;
            let old = std::mem::replace(
                &mut history.records[index],
                (Arc::clone(&record), wire_bytes),
            );
            retired.push(old.0);
        } else {
            history.records.push_back((Arc::clone(&record), wire_bytes));
        }
        history.wire_bytes += wire_bytes;
        while history.records.len() > CAPACITY || history.wire_bytes > WIRE_BUDGET {
            if let Some((old, bytes)) = history.records.pop_front() {
                history.wire_bytes -= bytes;
                history.evicted += 1;
                retired.push(old);
            }
        }
        drop(history);
        drop(retired);
        if let Form::List(fields) = &mut form {
            for pair in fields.chunks_exact_mut(2) {
                if pair[0] == Form::keyword("sequence") {
                    pair[1] = to_form(record.sequence.into());
                    break;
                }
            }
        }
        // A full publication queue only loses a notification: snapshots repair
        // missed updates without making the observed program await the broker.
        let _ = self
            .store
            .0
            .pubsub
            .try_publish(&TOPIC.into(), Val::from(form));
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
    async fn bounded_history_preserves_completions_during_snapshot_contention() {
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
        let mut completion = s.records[0].event.clone();
        completion.status = "returned".into();
        let id = completion.id.clone();
        let held = store.0.history.lock().unwrap();
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let (done_tx, done_rx) = std::sync::mpsc::channel();
        let writer = std::thread::spawn(move || {
            started_tx.send(()).unwrap();
            observer.observe(completion);
            done_tx.send(()).unwrap();
        });
        started_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        assert!(done_rx.recv_timeout(Duration::from_millis(20)).is_err());
        drop(held);
        done_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        writer.join().unwrap();
        let after = store.snapshot();
        assert_eq!(after.dropped, 0);
        assert_eq!(
            after
                .records
                .iter()
                .find(|r| r.event.id == id)
                .unwrap()
                .event
                .status,
            "returned"
        );
        // A snapshot remains a point-in-time copy while live records change.
        assert_eq!(s.records[0].event.status, "running");
    }
    #[tokio::test]
    async fn large_escaped_values_stay_below_the_connection_frame_limit() {
        let store = Store::new(crate::rt::pubsub::PubSub::spawn());
        let observer = store.observer("large-values".into());
        for i in 0..CAPACITY {
            observer.observe(Event {
                id: i.to_string(),
                parent: None,
                run: i.to_string(),
                kind: "call".into(),
                site: lyric::source::SourceSite::synthetic("(many_values)".into()),
                arguments: vec![
                    lyric::debug::Preview {
                        text: "\"".repeat(1024),
                        truncated: true
                    };
                    16
                ],
                arguments_truncated: false,
                result: None,
                status: "returned".into(),
                elapsed_us: 1,
                started_ms: 0,
            });
        }
        let snapshot = store.snapshot();
        assert!(snapshot.evicted > 0);
        assert!(snapshot.records.len() < CAPACITY);
        let form = to_form(serde_json::to_value(snapshot).unwrap());
        assert!(serde_json::to_vec(&form).unwrap().len() < WIRE_BUDGET + 4096);
    }
}
