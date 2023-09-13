//! Runtime implementation

use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;

use crate::machine;

/// Handle to runtime
#[derive(Debug)]
pub struct Runtime {
    /// Sender to backing runtime task
    evloop_tx: mpsc::Sender<Message>,
}

impl Runtime {
    /// Create a new runtime handle
    pub fn new() -> Self {
        let (evloop_tx, mut evloop_rx) = mpsc::channel(32);
        tokio::spawn(async move {
            let mut evloop = EventLoop::new();
            while let Some(msg) = evloop_rx.recv().await {
                evloop.handle_msg(msg);
            }
        });
        Self { evloop_tx }
    }

    /// Return number of connected clients in runtime
    pub async fn number_of_clients(&self) -> Result<usize> {
        let (req_tx, req_rx) = oneshot::channel();
        self.evloop_tx
            .send(Message::GetNumberOfClients(req_tx))
            .await
            .map_err(|_| Error::FailedToSendToEventLoop)?;
        Ok(req_rx.await?)
    }

    /// Dispatch an command to runtime
    pub async fn dispatch(&self, cmd: lemma::Form) -> Result<lemma::Value> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.evloop_tx
            .send(Message::DispatchCommand { cmd, resp_tx })
            .await
            .map_err(|_| Error::FailedToSendToEventLoop)?;
        Ok(resp_rx.await??)
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}

/// Messages passed between [Runtime] and [RuntimeTask] event loop
#[derive(Debug)]
pub enum Message {
    GetNumberOfClients(oneshot::Sender<usize>),
    DispatchCommand {
        cmd: machine::Command,
        resp_tx: oneshot::Sender<machine::Result>,
    },
}

/// The main event loop backing runtime
#[derive(Debug)]
struct EventLoop<'a> {
    /// The core machine
    machine: machine::Machine<'a>,

    /// Managed client tasks
    clients: Vec<JoinHandle<()>>,
}

impl EventLoop<'_> {
    fn new() -> Self {
        Self {
            machine: machine::Machine::new(),
            clients: vec![],
        }
    }

    /// Handle a message in event loop
    fn handle_msg(&mut self, msg: Message) {
        match msg {
            Message::GetNumberOfClients(resp_tx) => {
                let _ = resp_tx.send(self.clients.len());
            }
            Message::DispatchCommand { cmd, resp_tx } => {
                let _ = resp_tx.send(self.machine.dispatch(&cmd));
            }
        }
    }
}

/// Errors from [Runtime]
#[derive(thiserror::Error, Debug, PartialEq)]
pub enum Error {
    #[error("Failed to send message to event loop")]
    FailedToSendToEventLoop,

    #[error("Failed to receive response from event loop - {0}")]
    FailedToReceiveResponse(#[from] tokio::sync::oneshot::error::RecvError),

    #[error("Command returned error - {0}")]
    CommandError(#[from] machine::Error),
}

/// Result type for [Runtime]
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {

    use super::*;

    #[tokio::test]
    async fn runtime_init() {
        let runtime = Runtime::new();
        assert_eq!(runtime.number_of_clients().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn runtime_dispatch() {
        let runtime = Runtime::new();
        let form = lemma::parse("((lambda (x) x) \"hello world\")").unwrap();

        assert_eq!(
            runtime.dispatch(form).await,
            Ok(lemma::Value::from("hello world"))
        );
    }
}
