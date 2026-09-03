//! Local cache of service registrations, including registrations learned from nodes.

use nanoid::nanoid;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::{broadcast, mpsc, oneshot};

use lyric::{Form, KeywordId};
use tracing::error;

use crate::rt::program::Val;
use crate::{Error, Extern, ProcessExit, ProcessHandle, Result};

use super::ProcessId;

#[derive(Debug, Clone)]
pub struct Registry {
    tx: mpsc::Sender<Cmd>,
    events: broadcast::Sender<RegistryEvent>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct ServiceDescription {
    pub name: KeywordId,
    pub pid: ProcessId,
    pub interface: Vec<Form>,
    pub docs: HashMap<KeywordId, String>,
}

#[derive(Debug, Clone)]
pub(crate) enum RegistryEvent {
    Up(ServiceDescription),
    Down { name: KeywordId, pid: ProcessId },
}

#[derive(Debug)]
struct RegistryTask {
    weak_tx: mpsc::WeakSender<Cmd>,
    entries: HashMap<KeywordId, Vec<Entry>>,
    events: broadcast::Sender<RegistryEvent>,
    node_name: String,
    observed: u64,
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
}

impl Registry {
    pub(crate) fn spawn_named(node_name: String) -> Registry {
        let (tx, mut rx) = mpsc::channel(32);
        let (events, _) = broadcast::channel(64);
        let weak_tx = tx.downgrade();
        let task_events = events.clone();
        let task_node_name = node_name.clone();
        tokio::spawn(async move {
            let mut registry = RegistryTask::new(weak_tx, task_node_name, task_events);
            while let Some(cmd) = rx.recv().await {
                registry.handle_cmd(cmd).await
            }
        });
        Registry { tx, events }
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

    pub(crate) async fn local_snapshot(&self) -> Result<Vec<ServiceDescription>> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx
            .send(Cmd::LocalSnapshot(resp_tx))
            .await
            .map_err(|_| Error::NoMessageReceiver("registry task is dead".to_string()))?;
        resp_rx.await?
    }

    pub(crate) async fn replace_remote(
        &self,
        node: String,
        services: Vec<ServiceDescription>,
    ) -> Result<()> {
        self.tx
            .send(Cmd::ReplaceRemote(node, services))
            .await
            .map_err(|_| Error::NoMessageReceiver("registry task is dead".to_string()))
    }

    pub(crate) async fn remote_up(&self, service: ServiceDescription) -> Result<()> {
        self.tx
            .send(Cmd::RemoteUp(service))
            .await
            .map_err(|_| Error::NoMessageReceiver("registry task is dead".to_string()))
    }

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
        self.tx
            .send(Cmd::RemoveNode(node))
            .await
            .map_err(|_| Error::NoMessageReceiver("registry task is dead".to_string()))
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
        }
    }

    fn next_observed(&mut self) -> u64 {
        self.observed = self.observed.wrapping_add(1);
        self.observed
    }

    fn select_entry(entries: &[Entry]) -> Option<&Entry> {
        entries.iter().max_by_key(|entry| entry.observed)
    }

    async fn handle_cmd(&mut self, cmd: Cmd) {
        match cmd {
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
            Cmd::LocalSnapshot(resp_tx) => {
                let snapshot = self
                    .entries
                    .values()
                    .flatten()
                    .filter(|entry| entry.is_local())
                    .map(|entry| entry.description())
                    .collect();
                let _ = resp_tx.send(snapshot);
            }
            Cmd::ReplaceRemote(node, services) => {
                self.remove_node(&node);
                for service in services {
                    if service.pid.node() == node {
                        self.handle_remote_up(service);
                    }
                }
            }
            Cmd::RemoteUp(service) => self.handle_remote_up(service),
            Cmd::RemoteDown { node, name, pid } => self.remove_remote(&node, &name, &pid),
            Cmd::RemoveNode(node) => self.remove_node(&node),
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
            let _ = self.events.send(RegistryEvent::Up(description));
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
                let _ = self.events.send(RegistryEvent::Down {
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

    fn remove_remote(&mut self, node: &str, name: &KeywordId, pid: &ProcessId) {
        if let Some(entries) = self.entries.get_mut(name) {
            entries.retain(|entry| !entry.matches_remote(node, pid));
            if entries.is_empty() {
                self.entries.remove(name);
            }
        }
    }

    fn remove_node(&mut self, node: &str) {
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
}

enum Cmd {
    Register(Registration, ProcessHandle, oneshot::Sender<Result<()>>),
    Lookup(KeywordId, oneshot::Sender<Option<Entry>>),
    NotifyExit(KeywordId, EntryId, Result<ProcessExit>),
    GetAll(oneshot::Sender<Vec<Entry>>),
    LocalSnapshot(oneshot::Sender<Result<Vec<ServiceDescription>>>),
    ReplaceRemote(String, Vec<ServiceDescription>),
    RemoteUp(ServiceDescription),
    RemoteDown {
        node: String,
        name: KeywordId,
        pid: ProcessId,
    },
    RemoveNode(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{rt::kernel, Program};

    #[tokio::test]
    async fn remote_registration_replaces_service_from_same_node() {
        let registry = Registry::spawn_named("here".to_string());
        registry
            .remote_up(ServiceDescription {
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
                name: KeywordId::from("svc"),
                pid: ProcessId::new("one", 1),
                interface: vec![],
                docs: HashMap::new(),
            })
            .await
            .unwrap();
        registry
            .remote_up(ServiceDescription {
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
