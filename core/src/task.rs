//! Represents tasks running within runtime

use crate::{connection, runtime, Connection, Request, Response};
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

use tracing::{debug, error, info};

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
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Unable to send message to task")]
    UnableToSendMessage,

    #[error("Unable to communicate with hosting runtime")]
    UnableToMessageRuntime,

    #[error("Failed to receive response from runtime")]
    FailedToReceiveRuntimeResponse,

    #[error("Failed to send response on connection - {0}")]
    FailedToSendResponse(#[from] std::io::Error),
}

/// A join set for running tasks
pub type TaskSet = JoinSet<TaskId>;

impl Task {
    /// Create a new task in task set
    pub fn new(
        task_set: &mut TaskSet,
        id: TaskId,
        conn: Connection,
        runtime_tx: mpsc::WeakSender<runtime::Message>,
    ) -> Self {
        let (task_tx, mut task_rx) = mpsc::channel(32);
        let cancellation_token = CancellationToken::new();

        task_set.spawn(async move {
            let mut evloop = EventLoop {
                id,
                conn,
                runtime_tx,
                cancellation_token,
            };
            info!("Task {id} started");
            loop {
                let msg = tokio::select! {
                    msg = evloop.conn.recv() => match msg {
                        Some(Ok(msg)) => Message::from(msg),
                        Some(Err(e)) => Message::ConnRecvError(e),
                        None => {
                            info!("Task {id} terminating - Connection closed");
                            break
                        },
                    },
                    msg = task_rx.recv() => match msg {
                        Some(msg) => msg,
                        None => Message::HandleDropped,
                    },
                    _ = evloop.cancellation_token.cancelled() => {
                        info!("Task {id} terminating - Task cancelled");
                        break;
                    }
                };
                evloop.handle_msg(msg).await;
            }
            id
        });

        Self { id, task_tx }
    }

    /// Kill this task
    pub async fn kill(&self) -> Result<()> {
        info!("kill task {}", self.id);
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
    id: TaskId,
    conn: Connection,
    runtime_tx: mpsc::WeakSender<runtime::Message>,
    cancellation_token: CancellationToken,
}

impl EventLoop {
    async fn handle_msg(&mut self, msg: Message) {
        debug!("handle_msg - {msg:?}");
        match msg {
            Message::Kill => self.cancellation_token.cancel(),
            Message::RecvRequest(req) => {
                if let Err(e) = self.handle_req(req).await {
                    error!("Encountered error handling request - {e}");
                }
            }
            Message::RecvResponse(_) => todo!(),
            Message::HandleDropped => todo!(),
            Message::ConnRecvError(_) => todo!(),
        }
    }

    async fn handle_req(&mut self, req: Request) -> Result<()> {
        info!("Task {} request - {}", self.id, req.contents);

        let sender = match self.runtime_tx.upgrade() {
            Some(sender) => Ok(sender),
            None => Err(Error::UnableToMessageRuntime),
        }?;

        let (resp_tx, resp_rx) = oneshot::channel();

        sender
            .send(runtime::Message::DispatchCommand {
                cmd: req.contents,
                resp_tx,
            })
            .await
            .map_err(|_| Error::UnableToMessageRuntime)?;

        let resp = resp_rx
            .await
            .map_err(|_| Error::FailedToReceiveRuntimeResponse)?;

        let contents = match resp {
            Ok(lemma::Value::Form(f)) => f,
            Ok(_) => {
                // TODO - Is there better format for unserializable `Value` responses from runtime?
                lemma::Form::keyword("ok")
            }
            Err(e) => {
                error!("Error from evaluation - {e}");
                lemma::Form::keyword("err")
            }
        };

        // Always respond
        self.conn
            .send(&connection::Message::Response(Response {
                req_id: req.req_id,
                contents: contents.clone(),
            }))
            .await?;
        info!("Task {} result - {}", self.id, contents);

        Ok(())
    }
}

// TODO: Test cases for task
