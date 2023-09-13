//! Runtime implementation

use std::collections::HashMap;

use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinSet;

use tracing::{debug, error, trace};

use crate::connection::Connection;
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
            loop {
                tokio::select! {
                    Some(msg) = evloop_rx.recv() => {
                        evloop.handle_msg(msg).await;
                    },
                    Some(result) = evloop.tasks.join_next() => {
                        match result {
                            Ok(result) => evloop.handle_msg(Message::TaskEnded(result)).await,
                            Err(e) => error!("Task exited with error - {e}"),
                        }
                    }
                }
            }
        });
        Self { evloop_tx }
    }

    /// Notify runtime of new client connection
    pub async fn handle_conn(&self, conn: Connection) -> Result<()> {
        self.evloop_tx
            .send(Message::NewConnection(conn))
            .await
            .map_err(|_| Error::FailedToSendToEventLoop)?;
        Ok(())
    }

    /// Query set of tasks
    pub async fn list_tasks(&self) -> Result<Vec<TaskId>> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.evloop_tx
            .send(Message::ListTasks(resp_tx))
            .await
            .map_err(|_| Error::FailedToSendToEventLoop)?;
        Ok(resp_rx.await?)
    }

    /// Query number of tasks
    pub async fn task_count(&self) -> Result<usize> {
        Ok(self.list_tasks().await?.len())
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

    /// Kill the task specified by given task ID
    pub async fn kill_task(&self, id: &TaskId) -> Result<()> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.evloop_tx
            .send(Message::KillTask { id: *id, resp_tx })
            .await
            .map_err(|_| Error::FailedToSendToEventLoop)?;
        resp_rx.await?
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}

// ID assigned to tasks
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct TaskId(u32);

impl std::fmt::Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Messages passed between [Runtime] and [RuntimeTask] event loop
#[derive(Debug)]
pub enum Message {
    ListTasks(oneshot::Sender<Vec<TaskId>>),
    DispatchCommand {
        cmd: machine::Command,
        resp_tx: oneshot::Sender<machine::Result>,
    },
    NewConnection(Connection),
    TaskEnded(TaskEndResult),
    KillTask {
        id: TaskId,
        resp_tx: oneshot::Sender<Result<()>>,
    },
}

/// The main event loop backing runtime
#[derive(Debug)]
struct EventLoop<'a> {
    /// The core machine
    machine: machine::Machine<'a>,

    /// Counter for assigning IDs to tasks
    next_id: u32,

    /// Managed client tasks
    tasks: TaskSet,

    /// State to task from perspective of event loop
    task_handles: HashMap<TaskId, TaskHandle>,
}

impl EventLoop<'_> {
    fn new() -> Self {
        Self {
            next_id: 0,
            machine: machine::Machine::new(),
            tasks: TaskSet::new(),
            task_handles: HashMap::new(),
        }
    }

    /// Handle a message in event loop
    async fn handle_msg(&mut self, msg: Message) {
        trace!("handle_msg msg = {msg:?}");
        match msg {
            Message::ListTasks(resp_tx) => {
                let task_list = self.task_handles.keys().copied().collect();
                let _ = resp_tx.send(task_list);
            }
            Message::DispatchCommand { cmd, resp_tx } => {
                let _ = resp_tx.send(self.machine.dispatch(&cmd));
            }
            Message::NewConnection(conn) => {
                // TODO method-fy
                let tid = TaskId(self.next_id);
                self.next_id = self.next_id.wrapping_add(1);
                let handle = spawn_task(&mut self.tasks, tid, conn);
                self.task_handles.insert(tid, handle);
            }
            Message::TaskEnded(result) => {
                self.task_handles.remove(&result.id);
            }
            Message::KillTask { id, resp_tx } => {
                let _ = resp_tx.send(self.kill_task(id).await);
            }
        }
    }

    async fn kill_task(&self, id: TaskId) -> Result<()> {
        let handle = self
            .task_handles
            .get(&id)
            .ok_or(Error::UnrecognizedTaskId(id))?;
        handle.kill().await?;
        Ok(())
    }
}

/// Represents the state of a given task
#[derive(Debug, PartialEq)]
struct TaskStatus {}

/// Errors from [Runtime]
#[derive(thiserror::Error, Debug, PartialEq)]
pub enum Error {
    #[error("Failed to send message to event loop")]
    FailedToSendToEventLoop,

    #[error("Failed to receive response from event loop - {0}")]
    FailedToReceiveResponse(#[from] tokio::sync::oneshot::error::RecvError),

    #[error("Command returned error - {0}")]
    CommandError(#[from] machine::Error),

    #[error("Unrecognized task id - {0}")]
    UnrecognizedTaskId(TaskId),

    #[error("Task error - {0}")]
    TaskError(#[from] TaskError),
}

/// Result type for [Runtime]
pub type Result<T> = std::result::Result<T, Error>;

// TODO: This should be TaskResult<T> of some artifact T
/// The result of task
#[derive(Debug)]
pub struct TaskEndResult {
    id: TaskId,
}

/// Messages for task
#[derive(Debug)]
pub enum TaskMessage {
    Kill,
}

/// Errors for interacting with Task
#[derive(thiserror::Error, Debug, PartialEq)]
pub enum TaskError {
    #[error("Unable to send message to task")]
    UnableToSendMessage,
}

/// The handle to task
#[derive(Debug)]
pub struct TaskHandle {
    id: TaskId,
    task_tx: mpsc::Sender<TaskMessage>,
}

pub type TaskResult<T> = std::result::Result<T, TaskError>;

impl TaskHandle {
    pub async fn kill(&self) -> TaskResult<()> {
        debug!("kill task {}", self.id);
        self.task_tx
            .send(TaskMessage::Kill)
            .await
            .map_err(|_| TaskError::UnableToSendMessage)
    }
}

pub type TaskSet = JoinSet<TaskEndResult>;

/// Spawn a task for handling client connection
fn spawn_task(tasks: &mut TaskSet, id: TaskId, mut conn: Connection) -> TaskHandle {
    let (task_tx, mut task_rx) = mpsc::channel(32);
    tasks.spawn(async move {
        trace!("Started task {:?}", id);
        loop {
            tokio::select! {
                msg = conn.recv() => match msg {
                    Some(msg) => {
                        trace!("Client task received message from conn {:?}", msg);
                    }
                    None => {
                        trace!("Client task ending...");
                        break;
                    }
                },
                msg = task_rx.recv() => {
                    trace!("Client task received message {:?}", msg);
                    match msg {
                        Some(TaskMessage::Kill) => {
                            trace!("Killing task");
                            break;
                        },
                        None => {
                            trace!("Task handle dropped - killing task");
                        }
                    }
                }
            }
        }
        TaskEndResult { id }
    });
    TaskHandle { id, task_tx }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::connection::tests::conn_fixture;
    use tokio::time::error::Elapsed;
    use tracing_test::traced_test;

    #[tokio::test]
    #[traced_test]
    async fn runtime_init() {
        let runtime = Runtime::new();
        assert_eq!(runtime.task_count().await.unwrap(), 0);
    }

    #[tokio::test]
    #[traced_test]
    async fn runtime_dispatch() {
        let runtime = Runtime::new();
        let form = lemma::parse("((lambda (x) x) \"hello world\")").unwrap();

        assert_eq!(
            runtime.dispatch(form).await,
            Ok(lemma::Value::from("hello world"))
        );
    }

    #[tokio::test]
    #[traced_test]
    async fn runtime_handle_conn() {
        let runtime = Runtime::new();
        let (local, _remote) = conn_fixture();

        assert_eq!(runtime.handle_conn(local).await, Ok(()));
        assert_eq!(
            runtime.task_count().await.unwrap(),
            1,
            "There should be a connected client while remote is active"
        );
    }

    #[tokio::test]
    #[traced_test]
    async fn runtime_dropped_conn() {
        let runtime = Runtime::new();
        let (local, remote) = conn_fixture();

        runtime
            .handle_conn(local)
            .await
            .expect("Connection should be handled");
        wait_until_task_number(&runtime, 1)
            .await
            .expect("Accepting connection should increment task number");

        // Shutdown remote conn
        remote.shutdown().await.unwrap();

        wait_until_task_number(&runtime, 0)
            .await
            .expect("Dropped connection should decrement task number");

        assert_eq!(
            runtime.task_count().await.unwrap(),
            0,
            "Number of tasks should decrease"
        );
    }

    #[tokio::test]
    #[traced_test]
    async fn runtime_kill_task() {
        let runtime = Runtime::new();
        let (local, _remote) = conn_fixture();

        runtime
            .handle_conn(local)
            .await
            .expect("Connection should be handled");

        let tasks = runtime
            .list_tasks()
            .await
            .expect("Should be able to retrieve tasks");
        let task_id = tasks.first().expect("There should be at least one task");

        assert_eq!(runtime.kill_task(task_id).await, Ok(()));

        wait_until_task_number(&runtime, 0)
            .await
            .expect("Dropped connection should decrement task number");

        assert_eq!(
            runtime.task_count().await.unwrap(),
            0,
            "Number of tasks should decrease"
        );
    }

    /// Waits until the number of tasks in [Runtime] is [target]
    /// This is necessary since the select! poll between event loop Receiver and TaskSet is random
    async fn wait_until_task_number(
        runtime: &Runtime,
        target: usize,
    ) -> std::result::Result<(), Elapsed> {
        use std::time::Duration;
        use tokio::time::{sleep, timeout};

        let task_checker = async {
            loop {
                let current = runtime.task_count().await.unwrap();
                if current == target {
                    break;
                }
                sleep(Duration::from_millis(5)).await;
            }
        };

        timeout(Duration::from_millis(20), task_checker).await
    }
}
