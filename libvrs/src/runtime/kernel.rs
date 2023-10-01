#![allow(dead_code)] // TODO: Remove me

//! Runtime Kernel Task
use crate::runtime::v2::{Error, Result};
use crate::Connection;
use tokio::sync::{mpsc, oneshot};

/// Starts the kernel task
pub(crate) fn start() -> KernelHandle {
    let (msg_tx, mut msg_rx) = mpsc::channel(32);
    tokio::spawn(async move {
        let mut kernel = Kernel::new();
        while let Some(msg) = msg_rx.recv().await {
            kernel.handle_msg(msg).await?;
        }
        Ok::<(), Error>(())
    });

    KernelHandle { msg_tx }
}

/// Handle to `Kernel`
pub(crate) struct KernelHandle {
    msg_tx: mpsc::Sender<Message>,
}

impl KernelHandle {
    /// List running processes
    pub(crate) async fn list_processes(&self) -> Result<Vec<ProcessState>> {
        let (tx, rx) = oneshot::channel();
        self.msg_tx.send(Message::ListProcesses(tx)).await?;
        Ok(rx.await?)
    }

    /// Spawn a new process in runtime for given connection
    pub(crate) async fn spawn_proc_for_conn(&self, conn: Connection) -> Result<()> {
        self.msg_tx
            .send(Message::SpawnConnectionProcess(conn))
            .await?;
        Ok(())
    }
}

///
#[derive(Debug, PartialEq)]
pub struct ProcessState {
    // TBD
}

/// Messages for [Kernel]
#[derive(Debug)]
pub enum Message {
    /// List active processes
    ListProcesses(oneshot::Sender<Vec<ProcessState>>),
    /// Spawn a process to handle communciation with given channel
    SpawnConnectionProcess(Connection),
}

/// The runtime kernel task
struct Kernel {}

impl Kernel {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn handle_msg(&mut self, msg: Message) -> Result<()> {
        match msg {
            Message::ListProcesses(tx) => self.handle_list_process(tx).await,
            Message::SpawnConnectionProcess(_conn) => todo!(),
        }
    }

    async fn handle_list_process(&self, resp_tx: oneshot::Sender<Vec<ProcessState>>) -> Result<()> {
        let _ = resp_tx.send(vec![]); // Can fail if kernel task is shutting down
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn kernel_init() {
        let k = start();
        assert_eq!(k.list_processes().await.unwrap(), vec![]);
    }
}
