//! Represents tasks running within runtime

use crate::{connection, Connection, Request, Response};
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

use tracing::debug;

/// The handle to task
#[derive(Debug)]
pub struct Task {
    id: TaskId,
    task_tx: mpsc::Sender<Message>,
}

// ID assigned to tasks
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct TaskId(pub u32);

/// Result type for interactions with tasks
pub type Result<T> = std::result::Result<T, Error>;

/// Error for interactions of tasks
#[derive(thiserror::Error, Debug, PartialEq)]
pub enum Error {
    #[error("Unable to send message to task")]
    UnableToSendMessage,
}

/// A join set for running tasks
pub type TaskSet = JoinSet<TaskId>;

impl Task {
    /// Create a new task in task set
    pub fn new(task_set: &mut TaskSet, id: TaskId, mut conn: Connection) -> Self {
        let (task_tx, mut task_rx) = mpsc::channel(32);
        let cancellation_token = CancellationToken::new();

        task_set.spawn(async move {
            let mut evloop = EventLoop { cancellation_token };
            loop {
                let msg = tokio::select! {
                    msg = conn.recv() => match msg {
                        Some(Ok(msg)) => Message::from(msg),
                        Some(Err(e)) => Message::ConnRecvError(e),
                        None => Message::ConnectionClosed,
                    },
                    msg = task_rx.recv() => match msg {
                        Some(msg) => msg,
                        None => Message::HandleDropped,
                    },
                    _ = evloop.cancellation_token.cancelled() => {
                        break;
                    }
                };
                evloop.handle_msg(msg);
            }
            id
        });

        Self { id, task_tx }
    }

    /// Kill this task
    pub async fn kill(&self) -> Result<()> {
        debug!("kill task {}", self.id);
        self.task_tx
            .send(Message::Kill)
            .await
            .map_err(|_| Error::UnableToSendMessage)
    }
}

/// Messages handled by task's event loop
#[derive(Debug)]
enum Message {
    Kill,
    RecvRequest(Request),
    RecvResponse(Response),
    ConnectionClosed,
    HandleDropped,
    ConnRecvError(std::io::Error),
}

impl From<connection::Message> for Message {
    fn from(value: connection::Message) -> Self {
        match value {
            connection::Message::Request(req) => Self::RecvRequest(req),
            connection::Message::Response(resp) => Self::RecvResponse(resp),
        }
    }
}

/// Event loop for a task
struct EventLoop {
    cancellation_token: CancellationToken,
}

impl EventLoop {
    fn handle_msg(&mut self, msg: Message) {
        debug!("handle_msg - {msg:?}");
        match msg {
            Message::Kill => self.cancellation_token.cancel(),
            Message::ConnectionClosed => self.cancellation_token.cancel(),
            Message::RecvRequest(_) => todo!(),
            Message::RecvResponse(_) => todo!(),
            Message::HandleDropped => todo!(),
            Message::ConnRecvError(_) => todo!(),
        }
    }
}

// TODO: Test cases for task
