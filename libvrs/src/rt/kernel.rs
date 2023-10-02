#![allow(dead_code)] // TODO: Remove me

//! Runtime Kernel Task
use super::{
    process::{self, ProcessHandle, ProcessId, ProcessResult, ProcessSet},
    subscription::Subscription,
};
use crate::rt::{Error, Result};
use crate::Connection;
use std::collections::HashMap;
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, info};

/// Starts the kernel task
pub(crate) fn start() -> KernelHandle {
    let (msg_tx, mut msg_rx) = mpsc::channel(32);

    let handle = KernelHandle { msg_tx };
    let mut kernel = Kernel::new(handle.clone());
    tokio::spawn(async move {
        loop {
            tokio::select! {
                msg = msg_rx.recv() => match msg {
                    Some(msg) => kernel.handle_msg(msg).await?,
                    None => {
                        info!("Kernel handle dropped - terminating");
                        break;
                    }
                },
                Some(Ok(result)) = kernel.proc_set.join_next() => {
                    kernel.handle_msg(Message::ProcEnded(result)).await?;
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
    msg_tx: mpsc::Sender<Message>,
}

impl KernelHandle {
    /// List running processes
    pub(crate) async fn list_processes(&self) -> Result<Vec<ProcessId>> {
        let (tx, rx) = oneshot::channel();
        self.msg_tx.send(Message::ListProcesses(tx)).await?;
        rx.await
            .map_err(Error::FailedToReceiveResponseFromKernelTask)
    }

    // TODO Builder-fy?
    /// Spawn a new process
    pub(crate) async fn spawn_proc(&self, conn: Option<Connection>) -> Result<ProcessId> {
        let (tx, rx) = oneshot::channel();
        self.msg_tx.send(Message::SpawnProc(conn, tx)).await?;
        rx.await
            .map_err(Error::FailedToReceiveResponseFromKernelTask)
    }

    /// Get a process handle for given PID
    pub(crate) async fn get_proc(&self, id: ProcessId) -> Result<Option<ProcessHandle>> {
        let (tx, rx) = oneshot::channel();
        self.msg_tx.send(Message::GetProc(id, tx)).await?;
        let res = rx
            .await
            .map_err(Error::FailedToReceiveResponseFromKernelTask)?;
        Ok(res)
    }
}

/// Messages for [Kernel]
#[derive(Debug)]
pub enum Message {
    ListProcesses(oneshot::Sender<Vec<ProcessId>>),
    SpawnProc(Option<Connection>, oneshot::Sender<ProcessId>),
    GetProc(ProcessId, oneshot::Sender<Option<ProcessHandle>>),
    ProcEnded(ProcessResult),
}

/// The runtime kernel task
struct Kernel {
    handle: KernelHandle,
    procs: HashMap<ProcessId, ProcessHandle>,
    proc_set: ProcessSet,
    next_proc_id: usize,
}

impl Kernel {
    pub fn new(handle: KernelHandle) -> Self {
        Self {
            handle,
            procs: HashMap::new(),
            proc_set: ProcessSet::new(),
            next_proc_id: 0,
        }
    }

    pub async fn handle_msg(&mut self, msg: Message) -> Result<()> {
        debug!("handle_msg - {msg:?}");
        match msg {
            Message::ListProcesses(tx) => self.handle_list_process(tx),
            Message::SpawnProc(conn, rx) => {
                let id = self.spawn_proc(conn).await?;
                let _ = rx.send(id);
                Ok(())
            }
            Message::GetProc(id, rx) => {
                let _ = rx.send(self.get_proc(&id));
                Ok(())
            }
            Message::ProcEnded(result) => self.clean_proc(result),
        }
    }

    fn handle_list_process(&self, resp_tx: oneshot::Sender<Vec<ProcessId>>) -> Result<()> {
        let result = self.procs.keys().copied().collect();
        let _ = resp_tx.send(result); // Can fail if kernel task is shutting down
        Ok(())
    }

    /// Spawn a new process for given connection
    async fn spawn_proc(&mut self, conn: Option<Connection>) -> Result<ProcessId> {
        let id = ProcessId::from(self.next_proc_id);
        info!("spawn_proc {id:?}");

        self.next_proc_id = self.next_proc_id.wrapping_add(1);
        let p = process::spawn(id, &mut self.proc_set);
        if let Some(conn) = conn {
            p.add_subscription(Subscription::ClientConnection(conn))
                .await?;
        }
        self.procs.insert(id, p);

        Ok(id)
    }

    /// Retrieve the process handle for given id
    fn get_proc(&self, id: &ProcessId) -> Option<ProcessHandle> {
        self.procs.get(id).cloned()
    }

    /// Cleanup process that terminated with given result
    fn clean_proc(&mut self, result: ProcessResult) -> Result<()> {
        info!("clean_proc {:?}", result.proc_id);
        match self.procs.remove(&result.proc_id) {
            Some(_) => Ok(()),
            None => Err(Error::UnexpectedProcessResult),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{connection::tests::conn_fixture, Client};
    use lemma::{parse as p, Form};
    use std::time::Duration;
    use tokio::{task::yield_now, time::timeout};
    use tracing_test::traced_test;

    use super::*;

    #[tokio::test]
    #[traced_test]
    async fn kernel_init() {
        let k = start();
        assert_eq!(
            k.list_processes().await.unwrap().len(),
            0,
            "Should not have any processes on init"
        );

        assert!(!logs_contain("ERROR"));
    }

    #[tokio::test]
    #[traced_test]
    async fn kernel_proc_lifecycle() {
        let k = start();

        let pid = k
            .spawn_proc(None)
            .await
            .expect("Kernel should spawn new process");

        assert!(
            k.list_processes().await.unwrap().contains(&pid),
            "Kernel should have new process"
        );

        let proc = k
            .get_proc(pid)
            .await
            .expect("Kernel should respond")
            .expect("Handle should be Some");
        proc.shutdown().await.unwrap();

        timeout(Duration::from_secs(1), async {
            loop {
                if !k.list_processes().await.unwrap().contains(&pid) {
                    break;
                }
                yield_now().await;
            }
        })
        .await
        .expect("Kernel should be notified that process terminating and update state");

        assert!(!logs_contain("ERROR"));
    }

    #[tokio::test]
    #[traced_test]
    async fn kernel_proc_for_conn() {
        let (local, remote) = conn_fixture();
        let mut client = Client::new(remote);

        let k = start();

        let pid = k
            .spawn_proc(Some(local))
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

        // Verify via proc
        let proc = k
            .get_proc(pid)
            .await
            .expect("Kernel should return resp")
            .expect("Handle should not be none");

        assert_eq!(
            proc.call(p("msg").unwrap()).await.unwrap(),
            Form::string("Hello world")
        );

        assert!(!logs_contain("ERROR"));
    }
}
