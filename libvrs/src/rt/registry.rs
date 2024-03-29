//! Process Registry
use nanoid::nanoid;
use std::collections::HashMap;
use tokio::sync::{mpsc, oneshot};

use lyric::KeywordId;
use tracing::error;

use crate::rt::program::Val;
use crate::{Error, Extern, ProcessExit, ProcessHandle, Result};

use super::ProcessId;

/// Handle to [Registry]
#[derive(Debug, Clone)]
pub struct Registry {
    tx: mpsc::Sender<Cmd>,
}

/// Storage and lookup of processes
#[derive(Debug)]
pub struct RegistryTask {
    weak_tx: mpsc::WeakSender<Cmd>,
    entries: HashMap<KeywordId, Entry>,
}

/// Identifier for Entries
#[derive(Debug, Clone, PartialEq)]
pub struct EntryId(String);

/// Single entry in [Registry]
#[derive(Debug, Clone)]
pub struct Entry {
    id: EntryId,
    registration: Registration,
    handle: ProcessHandle,
}

/// Struct carrying registration payload
#[derive(Debug, Clone, PartialEq)]
pub struct Registration {
    keyword: KeywordId,
    interface: Vec<Val>,
    overwrite: bool,
    docs: HashMap<KeywordId, String>,
}

impl Registry {
    /// Spawn a new registry task
    pub fn spawn() -> Registry {
        let (tx, mut rx) = mpsc::channel(32);
        let weak_tx = tx.downgrade();
        tokio::spawn(async move {
            let mut registry = RegistryTask::new(weak_tx);
            while let Some(cmd) = rx.recv().await {
                registry.handle_cmd(cmd).await
            }
        });
        Registry { tx }
    }

    /// Register a given process
    pub async fn register(&self, registration: Registration, proc: ProcessHandle) -> Result<()> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx
            .send(Cmd::Register(registration, proc, resp_tx))
            .await
            .map_err(|_| Error::NoMessageReceiver("registry task is dead".to_string()))?;
        resp_rx.await?
    }

    /// Lookup given process for name
    pub async fn lookup(&self, keyword: KeywordId) -> Result<Option<Entry>> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx
            .send(Cmd::Lookup(keyword, resp_tx))
            .await
            .map_err(|_| Error::NoMessageReceiver("registry task is dead".to_string()))?;
        Ok(resp_rx.await?)
    }

    /// Get all entries
    pub async fn all(&self) -> Result<Vec<Entry>> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx
            .send(Cmd::GetAll(resp_tx))
            .await
            .map_err(|_| Error::NoMessageReceiver("registry task is dead".to_string()))?;
        Ok(resp_rx.await?)
    }
}

impl std::cmp::PartialEq for Registry {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(&self.tx, &other.tx)
    }
}

impl RegistryTask {
    fn new(weak_tx: mpsc::WeakSender<Cmd>) -> Self {
        Self {
            weak_tx,
            entries: HashMap::new(),
        }
    }

    async fn handle_cmd(&mut self, cmd: Cmd) {
        match cmd {
            Cmd::Register(registration, proc, resp_tx) => {
                let _ = resp_tx.send(self.handle_register(registration, proc));
            }
            Cmd::Lookup(keyword, resp_tx) => {
                let _ = resp_tx.send(self.entries.get(&keyword).cloned());
            }
            Cmd::NotifyExit(keyword, id, exit) => {
                self.handle_exit(keyword, id, exit);
            }
            Cmd::GetAll(resp_tx) => {
                let _ = resp_tx.send(self.entries.values().cloned().collect());
            }
        }
    }

    fn handle_register(&mut self, registration: Registration, handle: ProcessHandle) -> Result<()> {
        let keyword = &registration.keyword;

        if !registration.overwrite && self.entries.contains_key(keyword) {
            return Err(Error::RegistryError(format!(
                "Registered process exists for {}",
                keyword
            )));
        }

        let entry = Entry::new(registration.clone(), handle.clone());

        // Notify on exit
        let entry_id = entry.id.clone();
        let on_exit = handle.join();
        let weak_tx = self.weak_tx.clone();
        let kwd = keyword.clone();
        tokio::spawn(async move {
            let exit = on_exit.await;
            let tx = match weak_tx.upgrade() {
                Some(tx) => tx,
                None => return,
            };
            let _ = tx.send(Cmd::NotifyExit(kwd, entry_id, exit)).await;
        });

        self.entries.insert(keyword.clone(), entry);

        Ok(())
    }

    fn handle_exit(&mut self, keyword: KeywordId, id: EntryId, exit: Result<ProcessExit>) {
        match self.entries.get(&keyword) {
            Some(e) if e.id == id => {
                self.entries.remove(&keyword);
            }
            _ => {
                error!(
                    "handle_exit with unknown exit: {:?} {:?} {:?}",
                    keyword, id, exit
                );
            }
        };
    }
}

impl Entry {
    fn new(registration: Registration, handle: ProcessHandle) -> Self {
        Self {
            id: EntryId::from(nanoid!()),
            registration,
            handle,
        }
    }

    pub fn keyword(&self) -> &KeywordId {
        &self.registration.keyword
    }

    pub fn pid(&self) -> ProcessId {
        self.handle.id()
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
            Val::Extern(Extern::ProcessId(value.pid())),
        ];

        let interface = &value.registration.interface;
        if !interface.is_empty() {
            contents.push(Val::keyword("interface"));
            contents.push(Val::List(interface.clone()));
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

impl From<String> for EntryId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

enum Cmd {
    Register(Registration, ProcessHandle, oneshot::Sender<Result<()>>),
    Lookup(KeywordId, oneshot::Sender<Option<Entry>>),
    NotifyExit(KeywordId, EntryId, Result<ProcessExit>),
    GetAll(oneshot::Sender<Vec<Entry>>),
}

#[cfg(test)]
mod tests {
    use crate::{rt::kernel, Program};

    use super::*;
    use assert_matches::assert_matches;

    #[tokio::test]
    async fn empty() {
        let r = Registry::spawn();
        assert_matches!(
            r.lookup(KeywordId::from("unknown_keyword")).await.unwrap(),
            None
        );
        assert!(r.all().await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn register() {
        let r = Registry::spawn();
        let k = kernel::start();

        let prog = Program::from_expr("(loop (sleep 1))").unwrap();
        let hdl_a = k.spawn_prog(prog.clone()).await.unwrap();
        let hdl_b = k.spawn_prog(prog).await.unwrap();

        assert_matches!(r.lookup(KeywordId::from("A")).await.unwrap(), None);
        assert_matches!(r.lookup(KeywordId::from("B")).await.unwrap(), None);

        r.register(Registration::new(KeywordId::from("A")), hdl_a.clone())
            .await
            .expect("registration should succeed");
        r.register(Registration::new(KeywordId::from("B")), hdl_b.clone())
            .await
            .expect("registration should succeed");

        assert_matches!(r.lookup(KeywordId::from("A")).await.unwrap(),
                        Some(r) if r.handle.id() == hdl_a.id());
        assert_matches!(r.lookup(KeywordId::from("B")).await.unwrap(),
                        Some(r) if r.handle.id() == hdl_b.id());
    }

    #[tokio::test]
    async fn register_duplicate() {
        let r = Registry::spawn();
        let k = kernel::start();

        let prog = Program::from_expr("(loop (sleep 1))").unwrap();
        let hdl_a = k.spawn_prog(prog.clone()).await.unwrap();
        let hdl_b = k.spawn_prog(prog).await.unwrap();

        r.register(Registration::new(KeywordId::from("A")), hdl_a.clone())
            .await
            .expect("registration should succeed");

        assert_matches!(
            r.register(Registration::new(KeywordId::from("A")), hdl_b.clone())
                .await,
            Err(Error::RegistryError(_)),
            "Registration for existing key should fail"
        );
    }

    #[tokio::test]
    async fn register_duplicate_overwrite() {
        let r = Registry::spawn();
        let k = kernel::start();

        let prog = Program::from_expr("(loop (sleep 1))").unwrap();
        let hdl_a = k.spawn_prog(prog.clone()).await.unwrap();
        let hdl_b = k.spawn_prog(prog).await.unwrap();

        r.register(Registration::new(KeywordId::from("A")), hdl_a.clone())
            .await
            .expect("registration should succeed");

        let mut registration = Registration::new(KeywordId::from("A"));
        registration.overwrite(true);
        r.register(registration, hdl_b.clone())
            .await
            .expect("Registration for duplicate key should succeed since overwrite is true");

        assert_matches!(r.lookup(KeywordId::from("A")).await.unwrap(),
                        Some(r) if r.handle.id() == hdl_b.id(),
                        "Lookup should return newer registration");
    }

    #[tokio::test]
    async fn deregister_on_proc_exit() {
        let r = Registry::spawn();
        let k = kernel::start();

        let prog = Program::from_expr("(recv)").unwrap();
        let hdl = k.spawn_prog(prog.clone()).await.unwrap();

        r.register(Registration::new(KeywordId::from("A")), hdl.clone())
            .await
            .expect("registration should succeed");

        let _ = r
            .lookup(KeywordId::from("A"))
            .await
            .expect("process is still running - lookup should return Some");

        hdl.kill().await;
        hdl.join().await.expect("should complete");

        assert_matches!(
            r.lookup(KeywordId::from("A")).await.unwrap(),
            None,
            "Dead processes should be removed from registry"
        );
    }

    #[tokio::test]
    async fn get_all() {
        let r = Registry::spawn();
        let k = kernel::start();

        let prog = Program::from_expr("(loop (sleep 1))").unwrap();
        let hdl_a = k.spawn_prog(prog.clone()).await.unwrap();
        let hdl_b = k.spawn_prog(prog).await.unwrap();

        let mut reg_a = Registration::new(KeywordId::from("A"));
        reg_a.interface(vec![Val::keyword("interface_a")]);
        r.register(reg_a, hdl_a.clone())
            .await
            .expect("registration should succeed");

        let mut reg_b = Registration::new(KeywordId::from("B"));
        reg_b.interface(vec![Val::keyword("interface_b")]);
        r.register(reg_b, hdl_b.clone())
            .await
            .expect("registration should succeed");

        let entries: Vec<_> = r
            .all()
            .await
            .expect("Should be able to retrieve entries")
            .into_iter()
            .map(|e| (e.registration))
            .collect();
        assert!(entries.contains(
            Registration::new(KeywordId::from("A")).interface(vec![Val::keyword("interface_a")])
        ));
        assert!(entries.contains(
            Registration::new(KeywordId::from("B")).interface(vec![Val::keyword("interface_b")])
        ));

        hdl_a.kill().await;
        hdl_a.join().await.expect("should complete");

        let entries: Vec<_> = r
            .all()
            .await
            .expect("Should be able to retrieve entries")
            .into_iter()
            .map(|e| (e.registration.keyword))
            .collect();
        assert!(
            !entries.contains(&KeywordId::from("A")),
            "A should have been removed"
        );
        assert!(entries.contains(&KeywordId::from("B")));
    }
}
