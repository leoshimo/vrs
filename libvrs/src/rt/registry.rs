//! Local cache of service registrations, including registrations learned from nodes.

use nanoid::nanoid;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc, oneshot, watch};

use lyric::{Form, KeywordId};
use tracing::error;

use crate::rt::program::Val;
use crate::{Error, Extern, ProcessExit, ProcessHandle, Result};

use super::ProcessId;

// This bounds local bookkeeping, never user evaluation or service health.
const SYNC_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone)]
pub struct Registry {
    tx: mpsc::Sender<Cmd>,
    events: broadcast::Sender<RegistryEvent>,
    changed: watch::Sender<()>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ServiceDescription {
    pub name: KeywordId,
    pub pid: ProcessId,
    pub interface: Vec<Form>,
    pub docs: HashMap<KeywordId, String>,
    #[serde(default)]
    pub metadata: HashMap<KeywordId, Vec<Form>>,
    #[serde(default)]
    pub entity_completions: lyric::env::EntityCompletions,
}

#[derive(Debug, Clone)]
pub(crate) enum RegistryEvent {
    Up {
        revision: u64,
        service: ServiceDescription,
    },
    Down {
        revision: u64,
        name: KeywordId,
        pid: ProcessId,
    },
}

/// A checkpoint of this node's own registrations, taken by the registry actor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Snapshot {
    pub revision: u64,
    pub services: Vec<ServiceDescription>,
}

#[derive(Debug)]
pub(crate) enum RemoteChange {
    Snapshot(Snapshot),
    Up(u64, ServiceDescription),
    Down(u64, KeywordId, ProcessId),
}

#[derive(Debug)]
struct RegistryTask {
    weak_tx: mpsc::WeakSender<Cmd>,
    entries: HashMap<KeywordId, Vec<Entry>>,
    events: broadcast::Sender<RegistryEvent>,
    node_name: String,
    observed: u64,
    local_revision: u64,
    remote_revisions: HashMap<String, u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EntryId(String);

#[derive(Debug, Clone)]
pub struct Entry {
    id: EntryId,
    registration: Registration,
    target: EntryTarget,
    observed: u64,
}

#[derive(Debug, Clone)]
enum EntryTarget {
    Local(ProcessHandle),
    Remote(ProcessId),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Registration {
    keyword: KeywordId,
    interface: Vec<Val>,
    overwrite: bool,
    docs: HashMap<KeywordId, String>,
    metadata: HashMap<KeywordId, Vec<Form>>,
    entity_completions: lyric::env::EntityCompletions,
}

impl Registry {
    /// A live command channel with no processing task, for fault-injection tests.
    #[cfg(test)]
    pub(crate) fn stalled_for_test() -> (Self, impl Send) {
        let (tx, rx) = mpsc::channel(1);
        let (events, _) = broadcast::channel(1);
        let (changed, _) = watch::channel(());
        (
            Self {
                tx,
                events,
                changed,
            },
            rx,
        )
    }

    pub(crate) fn spawn_named(node_name: String) -> Registry {
        let (tx, mut rx) = mpsc::channel(32);
        let (events, _) = broadcast::channel(64);
        let (changed, _) = watch::channel(());
        let task_changed = changed.clone();
        let weak_tx = tx.downgrade();
        let task_events = events.clone();
        let task_node_name = node_name.clone();
        tokio::spawn(async move {
            let mut registry = RegistryTask::new(weak_tx, task_node_name, task_events);
            while let Some(cmd) = rx.recv().await {
                let changes = !matches!(
                    &cmd,
                    Cmd::Lookup(..) | Cmd::GetAll(..) | Cmd::Checkpoint(..)
                );
                registry.handle_cmd(cmd);
                if changes {
                    task_changed.send_replace(());
                }
            }
        });
        Registry {
            tx,
            events,
            changed,
        }
    }

    pub async fn register(&self, registration: Registration, proc: ProcessHandle) -> Result<()> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx
            .send(Cmd::Register(registration, proc, resp_tx))
            .await
            .map_err(|_| Error::NoMessageReceiver("registry task is dead".to_string()))?;
        resp_rx.await?
    }

    pub async fn lookup(&self, keyword: KeywordId) -> Result<Option<Entry>> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx
            .send(Cmd::Lookup(keyword, resp_tx))
            .await
            .map_err(|_| Error::NoMessageReceiver("registry task is dead".to_string()))?;
        Ok(resp_rx.await?)
    }

    pub async fn all(&self) -> Result<Vec<Entry>> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx
            .send(Cmd::GetAll(resp_tx))
            .await
            .map_err(|_| Error::NoMessageReceiver("registry task is dead".to_string()))?;
        Ok(resp_rx.await?)
    }

    pub(crate) fn subscribe(&self) -> broadcast::Receiver<RegistryEvent> {
        self.events.subscribe()
    }

    pub(crate) async fn wait_for(
        &self,
        name: KeywordId,
        pid: Option<ProcessId>,
    ) -> Result<ProcessId> {
        // Subscribe before checking to avoid missing a registration between them.
        let mut changed = self.changed.subscribe();
        loop {
            if let Some(entry) = self.lookup(name.clone()).await? {
                if pid.as_ref().is_none_or(|pid| *pid == entry.pid()) {
                    return Ok(entry.pid());
                }
            }
            changed
                .changed()
                .await
                .map_err(|_| Error::RegistryError("registry stopped while waiting".into()))?;
        }
    }

    /// Capture local entries and their revision in one registry command, after
    /// previously completed registrations. This does not wait for future child
    /// work or for another node. The internal deadline covers queueing and reply.
    pub(crate) async fn checkpoint(&self) -> Result<Snapshot> {
        self.synchronize("checkpoint", Cmd::Checkpoint).await
    }

    /// Acknowledges application, not just enqueueing: the registry mutates its
    /// in-memory entries before replying on the local oneshot. An already-seen
    /// revision is a successful no-op. This is not a network or health-check ack.
    /// Both remote replies and ordinary peer announcements await this operation.
    /// Channel closure and the internal deadline return errors, never success.
    /// Node-link replacement clears revisions from the previous runtime instance.
    pub(crate) async fn apply_remote(&self, node: String, change: RemoteChange) -> Result<()> {
        self.synchronize("apply remote update", |tx| {
            Cmd::ApplyRemote(node, change, tx)
        })
        .await
    }

    async fn synchronize<T>(
        &self,
        operation: &str,
        command: impl FnOnce(oneshot::Sender<Result<T>>) -> Cmd,
    ) -> Result<T> {
        tokio::time::timeout(SYNC_TIMEOUT, async {
            let (tx, rx) = oneshot::channel();
            self.tx.send(command(tx)).await.map_err(|_| {
                Error::RegistryError(format!("{operation}: registry channel closed"))
            })?;
            rx.await.map_err(|_| {
                Error::RegistryError(format!(
                    "{operation}: registry acknowledgement channel closed"
                ))
            })?
        })
        .await
        .map_err(|_| {
            Error::RegistryError(format!(
                "{operation}: registry did not acknowledge within {} seconds",
                SYNC_TIMEOUT.as_secs()
            ))
        })?
    }

    #[cfg(test)]
    pub(crate) async fn remote_up(&self, service: ServiceDescription) -> Result<()> {
        self.tx
            .send(Cmd::RemoteUp(service))
            .await
            .map_err(|_| Error::NoMessageReceiver("registry task is dead".to_string()))
    }

    #[cfg(test)]
    pub(crate) async fn remote_down(
        &self,
        node: String,
        name: KeywordId,
        pid: ProcessId,
    ) -> Result<()> {
        self.tx
            .send(Cmd::RemoteDown { node, name, pid })
            .await
            .map_err(|_| Error::NoMessageReceiver("registry task is dead".to_string()))
    }

    pub(crate) async fn remove_node(&self, node: String) -> Result<()> {
        self.synchronize("remove node", |tx| Cmd::RemoveNode(node, tx))
            .await
    }
}

impl PartialEq for Registry {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(&self.tx, &other.tx)
    }
}

impl RegistryTask {
    fn new(
        weak_tx: mpsc::WeakSender<Cmd>,
        node_name: String,
        events: broadcast::Sender<RegistryEvent>,
    ) -> Self {
        Self {
            weak_tx,
            entries: HashMap::new(),
            events,
            node_name,
            observed: 0,
            local_revision: 0,
            remote_revisions: HashMap::new(),
        }
    }

    fn next_observed(&mut self) -> u64 {
        self.observed = self.observed.wrapping_add(1);
        self.observed
    }

    fn select_entry(entries: &[Entry]) -> Option<&Entry> {
        entries.iter().max_by_key(|entry| entry.observed)
    }

    // Dependency invariant: handlers must not await user code, network I/O, or
    // the peer manager. The peer manager itself awaits checkpoint/apply replies;
    // waiting back on it would deadlock. Broadcast sends are nonblocking and
    // process joins run in separately spawned tasks. Keep this synchronous so
    // adding an await here requires an explicit change to the dependency model.
    fn handle_cmd(&mut self, cmd: Cmd) {
        match cmd {
            Cmd::Checkpoint(tx) => {
                let mut entries: Vec<_> = self
                    .entries
                    .values()
                    .flatten()
                    .filter(|e| e.is_local())
                    .collect();
                entries.sort_by_key(|e| e.observed);
                let services: Result<Vec<_>> =
                    entries.into_iter().map(Entry::description).collect();
                let _ = tx.send(services.map(|services| Snapshot {
                    revision: self.local_revision,
                    services,
                }));
            }
            Cmd::ApplyRemote(node, change, tx) => {
                // A timed-out response must not apply later and resurrect a
                // node's registrations after its link has been torn down.
                if !tx.is_closed() {
                    let _ = tx.send(self.apply_remote_change(node, change));
                }
            }
            Cmd::Register(registration, proc, resp_tx) => {
                let _ = resp_tx.send(self.handle_register(registration, proc));
            }
            Cmd::Lookup(keyword, resp_tx) => {
                let selected = self
                    .entries
                    .get(&keyword)
                    .and_then(|entries| Self::select_entry(entries))
                    .cloned();
                let _ = resp_tx.send(selected);
            }
            Cmd::NotifyExit(keyword, id, exit) => self.handle_exit(keyword, id, exit),
            Cmd::GetAll(resp_tx) => {
                let all = self
                    .entries
                    .values()
                    .filter_map(|entries| Self::select_entry(entries))
                    .cloned()
                    .collect();
                let _ = resp_tx.send(all);
            }
            #[cfg(test)]
            Cmd::RemoteUp(service) => self.handle_remote_up(service),
            #[cfg(test)]
            Cmd::RemoteDown { node, name, pid } => self.remove_remote(&node, &name, &pid),
            Cmd::RemoveNode(node, tx) => {
                // Cleanup remains useful even if its caller already timed out.
                self.remove_node(&node);
                let _ = tx.send(Ok(()));
            }
        }
    }

    fn handle_register(&mut self, registration: Registration, handle: ProcessHandle) -> Result<()> {
        let keyword = registration.keyword.clone();
        let existing_local = self
            .entries
            .get(&keyword)
            .is_some_and(|entries| entries.iter().any(Entry::is_local));
        if existing_local && !registration.overwrite {
            return Err(Error::RegistryError(format!(
                "Registered process exists for {keyword}"
            )));
        }

        if registration.overwrite {
            if let Some(entries) = self.entries.get_mut(&keyword) {
                entries.retain(|entry| !entry.is_local());
            }
        }

        let entry = Entry::local(registration, handle.clone(), self.next_observed());
        self.local_revision += 1;
        let entry_id = entry.id.clone();
        let on_exit = handle.join();
        let weak_tx = self.weak_tx.clone();
        let exit_keyword = keyword.clone();
        tokio::spawn(async move {
            let exit = on_exit.await;
            let Some(tx) = weak_tx.upgrade() else { return };
            let _ = tx.send(Cmd::NotifyExit(exit_keyword, entry_id, exit)).await;
        });

        if let Ok(description) = entry.description() {
            let _ = self.events.send(RegistryEvent::Up {
                revision: self.local_revision,
                service: description,
            });
        }
        self.entries.entry(keyword).or_default().push(entry);
        Ok(())
    }

    fn handle_exit(&mut self, keyword: KeywordId, id: EntryId, exit: Result<ProcessExit>) {
        let mut removed = None;
        if let Some(entries) = self.entries.get_mut(&keyword) {
            if let Some(index) = entries.iter().position(|entry| entry.id == id) {
                removed = Some(entries.remove(index));
            }
            if entries.is_empty() {
                self.entries.remove(&keyword);
            }
        }
        match removed {
            Some(entry) => {
                self.local_revision += 1;
                let _ = self.events.send(RegistryEvent::Down {
                    revision: self.local_revision,
                    name: keyword,
                    pid: entry.pid(),
                });
            }
            None => error!("handle_exit with unknown exit: {keyword:?} {id:?} {exit:?}"),
        }
    }

    fn handle_remote_up(&mut self, service: ServiceDescription) {
        if service.pid.node() == self.node_name {
            return;
        }
        let node = service.pid.node().to_string();
        let observed = self.next_observed();
        let keyword = service.name.clone();
        let entry = Entry::remote(service, observed);
        let entries = self.entries.entry(keyword).or_default();
        entries.retain(|entry| !entry.is_remote_on(&node));
        entries.push(entry);
    }

    fn apply_remote_change(&mut self, node: String, change: RemoteChange) -> Result<()> {
        let revision = match &change {
            RemoteChange::Snapshot(s) => s.revision,
            RemoteChange::Up(r, _) | RemoteChange::Down(r, _, _) => *r,
        };
        if self
            .remote_revisions
            .get(&node)
            .is_some_and(|old| *old >= revision)
        {
            return Ok(());
        }
        match change {
            RemoteChange::Snapshot(snapshot) => {
                // Reconcile rather than republish unchanged entries: refreshing
                // one node must not change which other node wins a shared name.
                self.entries.retain(|name, entries| {
                    entries.retain(|e| {
                        !e.is_remote_on(&node)
                            || snapshot
                                .services
                                .iter()
                                .any(|s| s.name == *name && s.pid == e.pid())
                    });
                    !entries.is_empty()
                });
                for service in snapshot.services {
                    if service.pid.node() != node {
                        continue;
                    }
                    let unchanged = self.entries.get(&service.name).is_some_and(|entries| {
                        entries.iter().any(|e| {
                            e.is_remote_on(&node) && e.description().ok().as_ref() == Some(&service)
                        })
                    });
                    if !unchanged {
                        self.handle_remote_up(service);
                    }
                }
            }
            RemoteChange::Up(_, service) => {
                if service.pid.node() != node {
                    return Err(Error::RegistryError(
                        "foreign service in node update".into(),
                    ));
                }
                self.handle_remote_up(service);
            }
            RemoteChange::Down(_, name, pid) => self.remove_remote(&node, &name, &pid),
        }
        self.remote_revisions.insert(node, revision);
        Ok(())
    }

    fn remove_remote(&mut self, node: &str, name: &KeywordId, pid: &ProcessId) {
        if let Some(entries) = self.entries.get_mut(name) {
            entries.retain(|entry| !entry.matches_remote(node, pid));
            if entries.is_empty() {
                self.entries.remove(name);
            }
        }
    }

    fn remove_node(&mut self, node: &str) {
        self.remote_revisions.remove(node);
        self.entries.retain(|_, entries| {
            entries.retain(|entry| entry.node() != node || entry.is_local());
            !entries.is_empty()
        });
    }
}

impl Entry {
    fn local(registration: Registration, handle: ProcessHandle, observed: u64) -> Self {
        Self {
            id: EntryId(nanoid!()),
            registration,
            target: EntryTarget::Local(handle),
            observed,
        }
    }

    fn remote(service: ServiceDescription, observed: u64) -> Self {
        let interface = service.interface.into_iter().map(Val::from).collect();
        Self {
            id: EntryId(nanoid!()),
            registration: Registration {
                keyword: service.name,
                interface,
                overwrite: false,
                docs: service.docs,
                metadata: service.metadata,
                entity_completions: service.entity_completions,
            },
            target: EntryTarget::Remote(service.pid),
            observed,
        }
    }

    fn description(&self) -> Result<ServiceDescription> {
        let interface = self
            .registration
            .interface
            .iter()
            .cloned()
            .map(Form::try_from)
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(Error::EvaluationError)?;
        Ok(ServiceDescription {
            name: self.registration.keyword.clone(),
            pid: self.pid(),
            interface,
            docs: self.registration.docs.clone(),
            metadata: self.registration.metadata.clone(),
            entity_completions: self.registration.entity_completions.clone(),
        })
    }

    fn is_local(&self) -> bool {
        matches!(self.target, EntryTarget::Local(_))
    }

    fn is_remote_on(&self, node: &str) -> bool {
        matches!(&self.target, EntryTarget::Remote(pid) if pid.node() == node)
    }

    fn matches_remote(&self, node: &str, pid: &ProcessId) -> bool {
        matches!(&self.target, EntryTarget::Remote(entry_pid) if entry_pid.node() == node && entry_pid == pid)
    }

    pub fn keyword(&self) -> &KeywordId {
        &self.registration.keyword
    }

    pub fn pid(&self) -> ProcessId {
        match &self.target {
            EntryTarget::Local(handle) => handle.id(),
            EntryTarget::Remote(pid) => pid.clone(),
        }
    }

    pub fn node(&self) -> &str {
        match &self.target {
            EntryTarget::Local(handle) => handle.id_ref().node(),
            EntryTarget::Remote(pid) => pid.node(),
        }
    }

    pub fn process_val(&self) -> Val {
        Val::Extern(Extern::ProcessId(self.pid()))
    }

    pub fn interface(&self) -> &Vec<Val> {
        &self.registration.interface
    }

    pub fn doc(&self, keyword: &KeywordId) -> Option<&String> {
        self.registration.docs.get(keyword)
    }

    pub fn metadata(&self, keyword: &KeywordId) -> Vec<Form> {
        self.registration
            .metadata
            .get(keyword)
            .cloned()
            .unwrap_or_default()
    }

    pub fn entity_completions(&self) -> &lyric::env::EntityCompletions {
        &self.registration.entity_completions
    }
}

impl From<Entry> for Val {
    fn from(value: Entry) -> Self {
        let mut contents = vec![
            Val::keyword("name"),
            Val::Keyword(value.keyword().clone()),
            Val::keyword("pid"),
            value.process_val(),
        ];
        if !value.is_local() {
            contents.push(Val::keyword("node"));
            contents.push(Val::String(value.node().to_string()));
        }
        if !value.registration.interface.is_empty() {
            contents.push(Val::keyword("interface"));
            contents.push(Val::List(value.registration.interface.clone()));
        }
        Val::List(contents)
    }
}

impl Registration {
    pub fn new(keyword: KeywordId) -> Self {
        Self {
            keyword,
            interface: vec![],
            overwrite: false,
            docs: HashMap::new(),
            metadata: HashMap::new(),
            entity_completions: HashMap::new(),
        }
    }

    pub fn overwrite(&mut self, overwrite: bool) -> &mut Self {
        self.overwrite = overwrite;
        self
    }

    pub fn interface(&mut self, interface: Vec<Val>) -> &mut Self {
        self.interface = interface;
        self
    }

    pub fn docs(&mut self, keyword: KeywordId, doc: String) -> &mut Self {
        self.docs.insert(keyword, doc);
        self
    }

    pub fn metadata(&mut self, keyword: KeywordId, metadata: Vec<Form>) {
        self.metadata.insert(keyword, metadata);
    }

    pub fn entity_completions(&mut self, completions: lyric::env::EntityCompletions) {
        self.entity_completions = completions;
    }
}

enum Cmd {
    Checkpoint(oneshot::Sender<Result<Snapshot>>),
    ApplyRemote(String, RemoteChange, oneshot::Sender<Result<()>>),
    Register(Registration, ProcessHandle, oneshot::Sender<Result<()>>),
    Lookup(KeywordId, oneshot::Sender<Option<Entry>>),
    NotifyExit(KeywordId, EntryId, Result<ProcessExit>),
    GetAll(oneshot::Sender<Vec<Entry>>),
    #[cfg(test)]
    RemoteUp(ServiceDescription),
    #[cfg(test)]
    RemoteDown {
        node: String,
        name: KeywordId,
        pid: ProcessId,
    },
    RemoveNode(String, oneshot::Sender<Result<()>>),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{rt::kernel, Program};

    #[tokio::test(start_paused = true)]
    async fn registry_sync_stall_errors_while_waiting_for_ack_and_queue_space() {
        use std::time::Duration;
        use tokio::time::timeout;
        let (registry, _receiver) = Registry::stalled_for_test();
        // First command fits but is never acknowledged. Subsequent commands
        // cannot even enqueue. Both phases, including disconnect cleanup, need
        // the internal deadline; the outer timeout is only the test watchdog.
        assert!(timeout(Duration::from_secs(10), registry.checkpoint())
            .await
            .expect("checkpoint hung")
            .is_err());
        assert!(timeout(
            Duration::from_secs(10),
            registry.apply_remote(
                "beta".into(),
                RemoteChange::Snapshot(Snapshot {
                    revision: 0,
                    services: vec![]
                })
            )
        )
        .await
        .expect("apply hung on a full queue")
        .is_err());
        assert!(
            timeout(Duration::from_secs(10), registry.remove_node("beta".into()))
                .await
                .expect("disconnect cleanup hung on a full queue")
                .is_err()
        );
    }

    #[tokio::test]
    async fn registry_sync_closed_channel_errors_without_waiting() {
        let (registry, receiver) = Registry::stalled_for_test();
        drop(receiver);
        assert!(registry.checkpoint().await.is_err());
        assert!(registry
            .apply_remote(
                "beta".into(),
                RemoteChange::Snapshot(Snapshot {
                    revision: 0,
                    services: vec![]
                })
            )
            .await
            .is_err());
    }

    #[tokio::test(start_paused = true)]
    async fn timed_out_snapshot_is_not_applied_when_registry_resumes() {
        let (tx, mut rx) = mpsc::channel(1);
        let (events, _) = broadcast::channel(1);
        let (changed, _) = watch::channel(());
        let registry = Registry {
            tx: tx.clone(),
            events: events.clone(),
            changed,
        };
        let mut task = RegistryTask::new(tx.downgrade(), "alpha".into(), events);
        let apply = registry.apply_remote(
            "beta".into(),
            RemoteChange::Snapshot(Snapshot {
                revision: 1,
                services: vec![description("probe", 1)],
            }),
        );
        assert!(
            tokio::time::timeout(std::time::Duration::from_secs(10), apply)
                .await
                .expect("apply hung")
                .is_err()
        );
        // The old response is still queued after the caller abandons it. It
        // must not resurrect remote entries when processing resumes.
        task.handle_cmd(rx.recv().await.unwrap());
        assert!(task.entries.is_empty());
        assert!(task.remote_revisions.is_empty());
    }

    fn description(name: &str, id: usize) -> ServiceDescription {
        ServiceDescription {
            name: name.into(),
            pid: ProcessId::new("beta", id),
            interface: vec![],
            docs: HashMap::new(),
            metadata: HashMap::new(),
            entity_completions: Default::default(),
        }
    }

    #[tokio::test]
    async fn versioned_checkpoint_prevents_stale_updates_and_resets_on_new_link() {
        let registry = Registry::spawn_named("alpha".into());
        registry
            .apply_remote(
                "beta".into(),
                RemoteChange::Snapshot(Snapshot {
                    revision: 5,
                    services: vec![description("probe", 2)],
                }),
            )
            .await
            .unwrap();
        registry
            .apply_remote("beta".into(), RemoteChange::Up(4, description("probe", 1)))
            .await
            .unwrap();
        assert_eq!(
            registry
                .lookup("probe".into())
                .await
                .unwrap()
                .unwrap()
                .pid(),
            ProcessId::new("beta", 2)
        );
        registry
            .apply_remote(
                "beta".into(),
                RemoteChange::Down(6, "probe".into(), ProcessId::new("beta", 2)),
            )
            .await
            .unwrap();
        registry
            .apply_remote(
                "beta".into(),
                RemoteChange::Snapshot(Snapshot {
                    revision: 5,
                    services: vec![description("probe", 2)],
                }),
            )
            .await
            .unwrap();
        assert!(registry.lookup("probe".into()).await.unwrap().is_none());
        registry.remove_node("beta".into()).await.unwrap();
        registry
            .apply_remote(
                "beta".into(),
                RemoteChange::Snapshot(Snapshot {
                    revision: 1,
                    services: vec![description("probe", 1)],
                }),
            )
            .await
            .unwrap();
        assert_eq!(
            registry
                .lookup("probe".into())
                .await
                .unwrap()
                .unwrap()
                .pid(),
            ProcessId::new("beta", 1)
        );
    }

    #[tokio::test]
    async fn checkpoint_refresh_does_not_republish_unchanged_service_over_another_node() {
        let registry = Registry::spawn_named("alpha".into());
        registry
            .apply_remote(
                "beta".into(),
                RemoteChange::Snapshot(Snapshot {
                    revision: 1,
                    services: vec![description("shared", 1)],
                }),
            )
            .await
            .unwrap();
        let mut other = description("shared", 8);
        other.pid = ProcessId::new("gamma", 8);
        registry
            .apply_remote("gamma".into(), RemoteChange::Up(1, other))
            .await
            .unwrap();
        registry
            .apply_remote(
                "beta".into(),
                RemoteChange::Snapshot(Snapshot {
                    revision: 2,
                    services: vec![description("shared", 1), description("unrelated", 2)],
                }),
            )
            .await
            .unwrap();
        assert_eq!(
            registry
                .lookup("shared".into())
                .await
                .unwrap()
                .unwrap()
                .pid(),
            ProcessId::new("gamma", 8)
        );
    }

    #[tokio::test]
    async fn remote_registration_replaces_service_from_same_node() {
        let registry = Registry::spawn_named("here".to_string());
        registry
            .remote_up(ServiceDescription {
                metadata: HashMap::new(),
                entity_completions: HashMap::new(),
                name: KeywordId::from("replaceable"),
                pid: ProcessId::new("remote", 1),
                interface: vec![Form::List(vec![
                    Form::keyword("first_hook"),
                    Form::symbol("cmd"),
                ])],
                docs: HashMap::new(),
            })
            .await
            .unwrap();
        registry
            .remote_up(ServiceDescription {
                metadata: HashMap::new(),
                entity_completions: HashMap::new(),
                name: KeywordId::from("replaceable"),
                pid: ProcessId::new("remote", 2),
                interface: vec![Form::List(vec![
                    Form::keyword("second_hook"),
                    Form::symbol("expr"),
                ])],
                docs: HashMap::new(),
            })
            .await
            .unwrap();

        let entries = registry.all().await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].pid(), ProcessId::new("remote", 2));
        assert_eq!(
            entries[0].interface(),
            &vec![Val::List(vec![
                Val::keyword("second_hook"),
                Val::symbol("expr"),
            ])]
        );

        registry
            .remote_down(
                "remote".to_string(),
                KeywordId::from("replaceable"),
                ProcessId::new("remote", 1),
            )
            .await
            .unwrap();
        let entries = registry.all().await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].pid(), ProcessId::new("remote", 2));
    }

    #[tokio::test]
    async fn latest_registration_wins_across_nodes() {
        let registry = Registry::spawn_named("here".to_string());
        registry
            .remote_up(ServiceDescription {
                metadata: HashMap::new(),
                entity_completions: HashMap::new(),
                name: KeywordId::from("svc"),
                pid: ProcessId::new("one", 1),
                interface: vec![],
                docs: HashMap::new(),
            })
            .await
            .unwrap();
        registry
            .remote_up(ServiceDescription {
                metadata: HashMap::new(),
                entity_completions: HashMap::new(),
                name: KeywordId::from("svc"),
                pid: ProcessId::new("two", 2),
                interface: vec![],
                docs: HashMap::new(),
            })
            .await
            .unwrap();
        let entries = registry.all().await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].node(), "two");
        assert_eq!(
            registry
                .lookup(KeywordId::from("svc"))
                .await
                .unwrap()
                .unwrap()
                .node(),
            "two"
        );

        let kernel = kernel::start_test();
        let handle = kernel
            .spawn_prog(Program::from_expr("(recv)").unwrap())
            .await
            .unwrap();
        registry
            .register(Registration::new(KeywordId::from("svc")), handle.clone())
            .await
            .unwrap();
        let entries = registry.all().await.unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].is_local());
        assert!(registry
            .lookup(KeywordId::from("svc"))
            .await
            .unwrap()
            .unwrap()
            .is_local());

        registry
            .remote_up(ServiceDescription {
                metadata: HashMap::new(),
                entity_completions: HashMap::new(),
                name: KeywordId::from("svc"),
                pid: ProcessId::new("three", 3),
                interface: vec![],
                docs: HashMap::new(),
            })
            .await
            .unwrap();
        let entries = registry.all().await.unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].node(), "three");
        assert_eq!(
            registry
                .lookup(KeywordId::from("svc"))
                .await
                .unwrap()
                .unwrap()
                .node(),
            "three"
        );
        handle.kill().await;
    }
}
