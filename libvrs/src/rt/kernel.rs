//! Runtime Kernel Task
use std::collections::HashMap;

use super::mailbox::Message;
use super::proc::{ProcessExit, ProcessHandle, ProcessSet};
use super::program;
use super::pubsub::{PubSub, PubSubHandle};
use super::registry::Registry;
use crate::rt::term::Term;
use crate::rt::{proc::Process, Error, ProcessId, Result};
use crate::{Connection, Program};
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
    pub(crate) async fn spawn_prog(&self, prog: Program) -> Result<ProcessHandle> {
        let (tx, rx) = oneshot::channel();
        self.ev_tx
            .send(Event::SpawnProg(prog, tx))
            .await
            .map_err(|_| Error::NoMessageReceiver("spawn failed".to_string()))?;
        rx.await
            .map_err(Error::FailedToReceiveResponseFromKernelTask)
    }

    /// Spawn a new process
    pub(crate) async fn spawn_for_conn(&self, conn: Connection) -> Result<ProcessHandle> {
        let (tx, rx) = oneshot::channel();
        self.ev_tx
            .send(Event::SpawnConnProc(conn, tx))
            .await
            .map_err(|_| Error::NoMessageReceiver("spawn_for_conn failed".to_string()))?;
        rx.await
            .map_err(Error::FailedToReceiveResponseFromKernelTask)
    }

    /// Get running process information
    pub(crate) async fn procs(&self) -> Result<Vec<ProcessId>> {
        let (tx, rx) = oneshot::channel();
        self.ev_tx
            .send(Event::ListProcess(tx))
            .await
            .map_err(|_| Error::NoMessageReceiver("procs failed".to_string()))?;
        rx.await
            .map_err(Error::FailedToReceiveResponseFromKernelTask)
    }

    /// Kill specified process
    pub(crate) async fn kill_proc(&self, pid: ProcessId) -> Result<()> {
        self.ev_tx
            .send(Event::KillProcess(pid))
            .await
            .map_err(|_| Error::NoMessageReceiver("kill_procs failed".to_string()))
    }

    // TODO(sec): SRC IDs too flexible
    /// Handle a message being sent from one process to another
    pub(crate) async fn send_message(
        &self,
        src: ProcessId,
        dst: ProcessId,
        val: program::Val,
    ) -> Result<()> {
        self.ev_tx
            .send(Event::ProcessSendMessage(src, dst, val))
            .await
            .map_err(|_| Error::NoMessageReceiver("send_message failed".to_string()))
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

impl std::cmp::PartialEq for WeakKernelHandle {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(&self.ev_tx, &other.ev_tx)
    }
}

/// Messages for [Kernel]
#[derive(Debug)]
pub enum Event {
    SpawnProg(Program, oneshot::Sender<ProcessHandle>),
    SpawnConnProc(Connection, oneshot::Sender<ProcessHandle>),
    ProcessExit(ProcessExit),
    ListProcess(oneshot::Sender<Vec<ProcessId>>),
    KillProcess(ProcessId),
    ProcessSendMessage(ProcessId, ProcessId, program::Val),
}

/// The runtime kernel task
struct Kernel {
    weak_hdl: WeakKernelHandle,
    procs: ProcessSet,
    proc_hdls: HashMap<ProcessId, ProcessHandle>,
    next_proc_id: usize,
    registry: Registry,
    pubsub: PubSubHandle,
}

impl Kernel {
    pub fn new(handle: KernelHandle) -> Self {
        Self {
            weak_hdl: handle.downgrade(),
            procs: ProcessSet::new(),
            proc_hdls: HashMap::new(),
            next_proc_id: 0,
            registry: Registry::spawn(),
            pubsub: PubSub::spawn(),
        }
    }

    pub async fn handle_ev(&mut self, ev: Event) -> Result<()> {
        debug!("handle_ev - {ev:?}");
        match ev {
            Event::SpawnProg(prog, tx) => {
                let proc = Process::from_prog(self.next_pid(), prog);
                let hdl = self.spawn(proc)?;
                let _ = tx.send(hdl);
                Ok(())
            }
            Event::SpawnConnProc(conn, tx) => {
                let proc = Process::from_prog(self.next_pid(), program::connection_program())
                    .term(Term::spawn(conn));
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
            Event::ProcessSendMessage(src, dst, msg) => self.dispatch_msg(src, dst, msg).await,
        }
    }

    /// Spawn a new process
    fn spawn(&mut self, proc: Process) -> Result<ProcessHandle> {
        let hdl = proc
            .kernel(self.weak_hdl.clone())
            .registry(self.registry.clone())
            .pubsub(self.pubsub.clone())
            .spawn(&mut self.procs)?;
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

    /// Dispatc message from src to dst
    async fn dispatch_msg(&self, src: ProcessId, dst: ProcessId, msg: program::Val) -> Result<()> {
        let dst = self.proc_hdls.get(&dst).ok_or(Error::UnknownProcess)?;
        dst.notify_message(Message::new(src, msg)).await;
        Ok(())
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
    use crate::{Client, Connection, ProcessResult};
    use assert_matches::assert_matches;
    use lyric::{parse as p, Form};
    use std::time::Duration;
    use tokio::time::timeout;

    use super::*;

    #[ignore] // TODO: Controlling terminal test
    #[tokio::test]
    async fn kernel_proc_for_conn() {
        let (local, remote) = Connection::pair().unwrap();
        let client = Client::new(remote);

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

    #[ignore] // TODO: Controlling terminal test
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

        assert_eq!(
            hdl.join().await.unwrap().status.unwrap(),
            ProcessResult::Disconnected
        );
        assert!(
            k.procs().await.unwrap().is_empty(),
            "Should terminate conn process for dropped conn"
        );
    }

    #[tokio::test]
    async fn kernel_spawn_kill() {
        let k = start();
        let hdl = k
            .spawn_prog(Program::from_expr("(loop (sleep 1))").unwrap())
            .await
            .expect("Kernel should spawn new process");
        assert_eq!(k.procs().await.unwrap(), vec![hdl.id()]);

        hdl.kill().await; // manual kill

        assert_eq!(
            hdl.join().await.unwrap().status.unwrap(),
            ProcessResult::Cancelled
        );
        assert!(
            k.procs().await.unwrap().is_empty(),
            "Should terminate conn process for killed process"
        );
    }

    #[tokio::test]
    async fn kernel_drop() {
        let k = start();
        let hdl = k
            .spawn_prog(Program::from_expr("(loop (sleep 0))").unwrap())
            .await
            .expect("Kernel should spawn new process");

        drop(k); // drop kernel

        let _ = timeout(Duration::from_millis(5), hdl.join())
            .await
            .expect("Process should terminate for dropped kernel before timeout");
    }

    #[tokio::test]
    async fn kernel_weak_handle() {
        let k = start();
        let weak_k = k.downgrade();
        let _ = k
            .spawn_prog(Program::from_expr("(loop (sleep 1))").unwrap())
            .await
            .expect("Kernel should spawn new process");

        assert_matches!(weak_k.upgrade(), Some(_));

        drop(k); // drop kernel

        assert_matches!(weak_k.upgrade(), None);
    }

    #[tokio::test]
    async fn kill_proc_from_kernel() {
        let k = start();
        let proc = k
            .spawn_prog(Program::from_expr("(loop (sleep 1))").unwrap())
            .await
            .expect("Kernel should spawn new process");

        k.kill_proc(proc.id()).await.unwrap();
        let exit = timeout(Duration::from_millis(5), proc.join())
            .await
            .expect("Process should terminate")
            .unwrap();

        assert_eq!(exit.status.unwrap(), ProcessResult::Cancelled);
        assert!(k.procs().await.unwrap().is_empty(),);
    }

    #[tokio::test]
    async fn kill_proc_from_proc() {
        use tokio::time;

        let k = start();

        let kill_target = k
            .spawn_prog(Program::from_expr("(loop (sleep 0))").unwrap())
            .await
            .unwrap();

        assert_eq!(
            k.procs().await.unwrap(),
            vec![kill_target.id()],
            "kernel procs should have running kill_target pid"
        );

        let kill_src = k
            .spawn_prog(
                Program::from_expr(&format!("(kill (pid {}))", kill_target.id().inner())).unwrap(),
            )
            .await
            .unwrap();

        kill_src.join().await.expect("kill_src should terminate");

        let killed_exit = time::timeout(Duration::from_millis(0), kill_target.join())
            .await
            .expect("kill_target process should terminate")
            .unwrap();
        assert_eq!(killed_exit.status.unwrap(), ProcessResult::Cancelled);

        assert_eq!(
            k.procs().await.unwrap(),
            vec![],
            "kernel proc should no longer track kill_target"
        );
    }

    #[tokio::test]
    async fn spawn_progs() {
        let k = start();

        let recv = k
            .spawn_prog(Program::from_expr("(recv)").unwrap())
            .await
            .unwrap();

        assert_eq!(k.procs().await.unwrap(), vec![recv.id()],);

        let send_prog = format!("(send (pid {}) :hi)", recv.id().inner());
        let send = k
            .spawn_prog(Program::from_expr(&send_prog).unwrap())
            .await
            .unwrap();

        assert_eq!(
            send.join().await.unwrap().status.unwrap(),
            ProcessResult::Done(program::Val::keyword("hi"))
        );
        assert_eq!(
            recv.join().await.unwrap().status.unwrap(),
            ProcessResult::Done(program::Val::keyword("hi"))
        );

        assert!(k.procs().await.unwrap().is_empty());
    }
}
