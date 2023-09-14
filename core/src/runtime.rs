//! Runtime implementation

use std::collections::HashMap;

use tokio::sync::{mpsc, oneshot};

use crate::task::{self, Task, TaskId, TaskSet};
use tracing::{error, trace};

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
        let evloop_tx_weak = evloop_tx.downgrade();

        tokio::spawn(async move {
            let mut evloop = EventLoop::new(evloop_tx_weak);
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
    TaskEnded(TaskId),
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
    task_handles: HashMap<TaskId, Task>,

    /// Weak sender to give to spawned task to notify back runtime
    evloop_tx: mpsc::WeakSender<Message>,
}

impl EventLoop<'_> {
    fn new(evloop_tx: mpsc::WeakSender<Message>) -> Self {
        Self {
            next_id: 0,
            machine: machine::Machine::new(),
            tasks: TaskSet::new(),
            task_handles: HashMap::new(),
            evloop_tx,
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
                self.spawn_task(conn);
            }
            Message::TaskEnded(result) => {
                self.task_handles.remove(&result);
            }
            Message::KillTask { id, resp_tx } => {
                let _ = resp_tx.send(self.kill_task(id).await);
            }
        }
    }

    /// Handle new connection in event loop
    fn spawn_task(&mut self, conn: Connection) {
        let id = TaskId(self.next_id);
        self.next_id = self.next_id.wrapping_add(1);
        let task = Task::new(&mut self.tasks, id, conn, self.evloop_tx.clone());
        trace!("Started task {:?}", task);
        self.task_handles.insert(id, task);
    }

    /// Kill the task with given ID
    async fn kill_task(&self, id: TaskId) -> Result<()> {
        let handle = self
            .task_handles
            .get(&id)
            .ok_or(Error::UnrecognizedTaskId(id))?;
        handle.kill().await?;
        Ok(())
    }
}

/// Errors from [Runtime]
#[derive(thiserror::Error, Debug)]
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
    TaskError(#[from] task::Error),
}

/// Result type for [Runtime]
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {

    use super::*;
    use crate::connection::tests::conn_fixture;
    use crate::{Client, Response};
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

        assert!(matches!(
            runtime
                .dispatch(form)
                .await,
            Ok(f) if f == lemma::Value::from("hello world")
        ));
    }

    #[tokio::test]
    #[traced_test]
    async fn runtime_handle_conn() {
        let runtime = Runtime::new();
        let (local, _remote) = conn_fixture();

        assert!(matches!(runtime.handle_conn(local).await, Ok(())));
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

        assert!(matches!(runtime.kill_task(task_id).await, Ok(())));

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
    async fn runtime_remote_request_simple() {
        use crate::connection::Message;
        use crate::{Request, Response};

        let runtime = Runtime::new();
        let (local, mut remote) = conn_fixture();

        runtime
            .handle_conn(local)
            .await
            .expect("Connection should be handled");

        // TODO: Use =client::Client=?
        assert!(
            matches!(
                remote
                    .send(&Message::Request(Request {
                        req_id: 0,
                        contents: lemma::Form::string("Hello world"),
                    }))
                    .await,
                Ok(())
            ),
            "Sending request should succeed"
        );

        let resp = remote
            .recv()
            .await
            .expect("Remote should be open")
            .expect("Read should succeed");

        assert_eq!(
            resp,
            Message::Response(Response {
                req_id: 0,
                contents: lemma::Form::List(vec![
                    lemma::Form::keyword("ok"),
                    lemma::Form::string("Hello world"),
                ]),
            })
        );
    }

    #[tokio::test]
    #[traced_test]
    async fn runtime_remote_request_multi() {
        use lemma::parse as p;

        let runtime = Runtime::new();
        let (local, remote) = conn_fixture();
        let mut client = Client::new(remote);

        runtime
            .handle_conn(local)
            .await
            .expect("Connection should be handled");

        assert!(
            matches!(
                client.request(p("(define echo (lambda (x) x))").unwrap()).await,
                Ok(Response { contents, .. }) if contents == lemma::Form::List(vec![
                    lemma::Form::keyword("ok"),
                    lemma::Form::string("<lambda (x)>"),
                ])
            ),
            "defining a function should return :ok"
        );
        assert!(
            matches!(
                client.request(p("echo").unwrap()).await,
                Ok(Response { contents, .. }) if contents == lemma::Form::List(vec![
                    lemma::Form::keyword("ok"),
                    lemma::Form::string("<lambda (x)>"),
                ])
            ),
            "evaluating function symbol should return :ok"
        );
        assert!(
            matches!(
                client.request(p("(echo \"Hello world\")").unwrap()).await,
                Ok(Response { contents, .. }) if contents == lemma::Form::List(vec![
                    lemma::Form::keyword("ok"),
                    lemma::Form::string("Hello world"),
                ])
            ),
            "evaluating a function call should return result"
        );
        assert!(
            matches!(
                client.request(p("jibberish").unwrap()).await,
                Ok(Response { contents, .. }) if contents == lemma::Form::List(vec![
                    lemma::Form::keyword("err"),
                    lemma::Form::string("Undefined symbol - jibberish"),
                ])
            ),
            "evaluating a jibberish underined symbol should return :err"
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
