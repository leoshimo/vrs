#![allow(dead_code)]

//! Runtime Kernel Task
use std::collections::HashMap;

use super::proc::{self, ProcessExit, ProcessHandle, ProcessSet};
use crate::rt::{proc::Process, Error, ProcessId, Result};
use crate::Connection;
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, info};

/// Handle to `Kernel`
#[derive(Debug, Clone)]
pub(crate) struct KernelHandle {
    ev_tx: mpsc::Sender<Event>,
}

/// Handle to `Kernel`
#[derive(Debug, Clone)]
pub(crate) struct WeakKernelHandle {
    ev_tx: mpsc::WeakSender<Event>,
}

/// Starts the kernel task, which manages processes on runtime
pub(crate) fn start() -> KernelHandle {
    let (ev_tx, mut ev_rx) = mpsc::channel(32);

    let handle = KernelHandle { ev_tx };
    let mut kernel = Kernel::new(handle.clone());
    tokio::spawn(async move {
        loop {
            tokio::select! {
                ev = ev_rx.recv() => match ev {
                    Some(ev) => kernel.handle_ev(ev).await?,
                    None => {
                        info!("Kernel handle dropped - terminating");
                        break;
                    }
                },
                Some(Ok(result)) = kernel.procs.join_next() => {
                    kernel.handle_ev(Event::ProcessExit(result)).await?;
                }
            }
        }
        Ok::<(), Error>(())
    });
    handle
}

impl KernelHandle {
    /// Spawn a new program
    pub(crate) async fn spawn_prog(&self, prog: proc::Val) -> Result<ProcessHandle> {
        let (tx, rx) = oneshot::channel();
        self.ev_tx
            .send(Event::SpawnProg(prog, tx))
            .await
            .map_err(|_| Error::FailedToMessageKernel("spawn failed".to_string()))?;
        rx.await
            .map_err(Error::FailedToReceiveResponseFromKernelTask)
    }

    /// Spawn a new process
    pub(crate) async fn spawn_for_conn(&self, conn: Connection) -> Result<ProcessHandle> {
        let (tx, rx) = oneshot::channel();
        self.ev_tx
            .send(Event::SpawnConnProc(conn, tx))
            .await
            .map_err(|_| Error::FailedToMessageKernel("spawn_for_conn failed".to_string()))?;
        rx.await
            .map_err(Error::FailedToReceiveResponseFromKernelTask)
    }

    /// Get running process information
    pub(crate) async fn procs(&self) -> Result<Vec<ProcessId>> {
        let (tx, rx) = oneshot::channel();
        self.ev_tx
            .send(Event::ListProcess(tx))
            .await
            .map_err(|_| Error::FailedToMessageKernel("procs failed".to_string()))?;
        rx.await
            .map_err(Error::FailedToReceiveResponseFromKernelTask)
    }

    /// Kill specified process
    pub(crate) async fn kill_proc(&self, pid: ProcessId) -> Result<()> {
        self.ev_tx
            .send(Event::KillProcess(pid))
            .await
            .map_err(|_| Error::FailedToMessageKernel("kill_procs failed".to_string()))
    }

    /// Downgrade a strong kernel handle to weak handle
    pub(crate) fn downgrade(&self) -> WeakKernelHandle {
        WeakKernelHandle {
            ev_tx: self.ev_tx.downgrade(),
        }
    }
}

impl WeakKernelHandle {
    /// Update a weak process handle into strong ref
    pub(crate) fn upgrade(&self) -> Option<KernelHandle> {
        let ev_tx = self.ev_tx.upgrade()?;
        Some(KernelHandle { ev_tx })
    }
}

/// Messages for [Kernel]
#[derive(Debug)]
pub enum Event {
    SpawnProg(proc::Val, oneshot::Sender<ProcessHandle>),
    SpawnConnProc(Connection, oneshot::Sender<ProcessHandle>),
    ProcessExit(ProcessExit),
    ListProcess(oneshot::Sender<Vec<ProcessId>>),
    KillProcess(ProcessId),
}

/// The runtime kernel task
struct Kernel {
    weak_hdl: WeakKernelHandle,
    procs: ProcessSet,
    proc_hdls: HashMap<ProcessId, ProcessHandle>,
    next_proc_id: usize,
}

impl Kernel {
    pub fn new(handle: KernelHandle) -> Self {
        Self {
            weak_hdl: handle.downgrade(),
            procs: ProcessSet::new(),
            proc_hdls: HashMap::new(),
            next_proc_id: 0,
        }
    }

    pub async fn handle_ev(&mut self, ev: Event) -> Result<()> {
        debug!("handle_ev - {ev:?}");
        match ev {
            Event::SpawnProg(prog, tx) => {
                let proc = Process::from_val(self.next_pid(), prog)?;
                let hdl = self.spawn(proc)?;
                let _ = tx.send(hdl);
                Ok(())
            }
            Event::SpawnConnProc(conn, tx) => {
                let proc =
                    Process::from_expr(self.next_pid(), "(loop (send_resp (peval (recv_req))))")
                        .unwrap()
                        .conn(conn);
                let hdl = self.spawn(proc)?;
                let _ = tx.send(hdl);
                Ok(())
            }
            Event::ProcessExit(exit) => self.handle_exit(exit),
            Event::ListProcess(tx) => {
                let ids = self.proc_hdls.keys().copied().collect();
                let _ = tx.send(ids);
                Ok(())
            }
            Event::KillProcess(pid) => self.kill_proc(pid).await,
        }
    }

    /// Spawn a new process
    fn spawn(&mut self, proc: Process) -> Result<ProcessHandle> {
        let hdl = proc.kernel(self.weak_hdl.clone()).spawn(&mut self.procs)?;
        self.proc_hdls.insert(hdl.id(), hdl.clone());
        Ok(hdl)
    }

    /// Cleanup process that terminated with given result
    fn handle_exit(&mut self, exit: ProcessExit) -> Result<()> {
        match self.proc_hdls.remove(&exit.id) {
            Some(_) => Ok(()),
            None => panic!("Kernel notified of unmanaged process"),
        }
    }

    /// Kill specified process
    async fn kill_proc(&self, pid: ProcessId) -> Result<()> {
        match self.proc_hdls.get(&pid) {
            Some(hdl) => {
                hdl.kill().await;
                Ok(())
            }
            None => Err(Error::UnknownProcess),
        }
    }

    /// Get the next process id
    fn next_pid(&mut self) -> ProcessId {
        let id = ProcessId::from(self.next_proc_id);
        self.next_proc_id = self.next_proc_id.wrapping_add(1);
        id
    }
}

#[cfg(test)]
mod tests {
    use crate::{Client, Connection, Request};
    use assert_matches::assert_matches;
    use lyric::{parse as p, Form};
    use std::time::Duration;
    use tokio::time::timeout;

    use super::*;

    #[tokio::test]
    async fn kernel_proc_for_conn() {
        let (local, remote) = Connection::pair().unwrap();
        let mut client = Client::new(remote);

        let k = start();
        let _ = k
            .spawn_for_conn(local)
            .await
            .expect("Kernel should spawn new process");

        let _ = client
            .request(p("(def msg \"Hello world\")").unwrap())
            .await
            .expect("Client should send request");

        // Verify via client
        let resp = client
            .request(p("msg").unwrap())
            .await
            .expect("Client should send request");
        assert_eq!(resp.contents, Ok(Form::string("Hello world")));
    }

    #[tokio::test]
    async fn kernel_spawn_conn_drop() {
        let (local, remote) = Connection::pair().unwrap();

        let k = start();
        let hdl = k
            .spawn_for_conn(local)
            .await
            .expect("Kernel should spawn new process");
        assert_eq!(k.procs().await.unwrap(), vec![hdl.id()]);

        drop(remote); // remote terminates
        hdl.join().await;

        assert!(
            k.procs().await.unwrap().is_empty(),
            "Should terminate conn process for dropped conn"
        );
    }

    #[tokio::test]
    async fn kernel_spawn_kill() {
        let (local, _remote) = Connection::pair().unwrap();
        let k = start();
        let hdl = k
            .spawn_for_conn(local)
            .await
            .expect("Kernel should spawn new process");
        assert_eq!(k.procs().await.unwrap(), vec![hdl.id()]);

        hdl.kill().await; // manual kill
        hdl.join().await;

        assert!(
            k.procs().await.unwrap().is_empty(),
            "Should terminate conn process for killed process"
        );
    }

    #[tokio::test]
    async fn kernel_drop() {
        let (local, _remote) = Connection::pair().unwrap();
        let k = start();
        let hdl = k
            .spawn_for_conn(local)
            .await
            .expect("Kernel should spawn new process");

        drop(k); // drop kernel

        let proc_exit = tokio::spawn(async move {
            hdl.join().await;
        });
        let _ = timeout(Duration::from_millis(5), proc_exit)
            .await
            .expect("Process should terminate for dropped kernel before timeout");
    }

    #[tokio::test]
    async fn kernel_weak_handle() {
        let (local, _remote) = Connection::pair().unwrap();
        let k = start();
        let weak_k = k.downgrade();
        let _ = k
            .spawn_for_conn(local)
            .await
            .expect("Kernel should spawn new process");

        assert_matches!(weak_k.upgrade(), Some(_));

        drop(k); // drop kernel

        assert_matches!(weak_k.upgrade(), None);
    }

    #[tokio::test]
    async fn kill_proc_from_kernel() {
        let (local, _remote) = Connection::pair().unwrap();
        let k = start();
        let proc = k
            .spawn_for_conn(local)
            .await
            .expect("Kernel should spawn new process");

        k.kill_proc(proc.id()).await.unwrap();
        let proc_exit = tokio::spawn(async move {
            proc.join().await;
        });

        let _ = timeout(Duration::from_millis(5), proc_exit)
            .await
            .expect("Process should terminate");

        assert!(
            k.procs().await.unwrap().is_empty(),
        );
    }

    #[tokio::test]
    async fn kill_proc_from_proc() {
        let (local, mut remote) = Connection::pair().unwrap();
        let (local_other, _remote_other) = Connection::pair().unwrap();
        let k = start();

        let proc = k.spawn_for_conn(local).await.unwrap();

        let proc_other = k.spawn_for_conn(local_other).await.unwrap();

        remote
            .send_req(Request {
                req_id: 0,
                contents: lyric::parse(&format!("(kill {})", proc_other.id())).unwrap(),
            })
            .await
            .unwrap();

        let proc_exit = tokio::spawn(async move {
            proc_other.join().await;
        });
        let _ = timeout(Duration::from_millis(5), proc_exit)
            .await
            .expect("Killed process should terminate");

        assert_eq!(
            k.procs().await.unwrap(),
            vec![proc.id()],
            "Only remaining process should be tracked by kernel",
        );
    }
}
