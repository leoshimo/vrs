//! Runtime Kernel Task
use std::collections::HashMap;

use super::proc::{self, ProcessExit, ProcessHandle, ProcessSet};
use crate::rt::{proc::Process, Error, ProcessId, Result};
use crate::Connection;
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, info};

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

/// Handle to `Kernel`
#[derive(Clone)]
pub(crate) struct KernelHandle {
    ev_tx: mpsc::Sender<Event>,
}

#[allow(dead_code)]
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
}

/// Messages for [Kernel]
#[derive(Debug)]
pub enum Event {
    SpawnProg(proc::Val, oneshot::Sender<ProcessHandle>),
    SpawnConnProc(Connection, oneshot::Sender<ProcessHandle>),
    ProcessExit(ProcessExit),
    ListProcess(oneshot::Sender<Vec<ProcessId>>),
}

/// The runtime kernel task
struct Kernel {
    procs: ProcessSet,
    proc_hdls: HashMap<ProcessId, ProcessHandle>,
    next_proc_id: usize,
}

impl Kernel {
    pub fn new(_handle: KernelHandle) -> Self {
        Self {
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
        }
    }

    /// Spawn a new process
    fn spawn(&mut self, proc: Process) -> Result<ProcessHandle> {
        let hdl = proc.spawn(&mut self.procs)?;
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

    /// Get the next process id
    fn next_pid(&mut self) -> ProcessId {
        let id = ProcessId::from(self.next_proc_id);
        self.next_proc_id = self.next_proc_id.wrapping_add(1);
        id
    }
}

#[cfg(test)]
mod tests {
    use crate::{Client, Connection};
    use lyric::{parse as p, Form};

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
    #[tracing_test::traced_test]
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
}
