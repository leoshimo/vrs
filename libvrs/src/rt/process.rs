//! Runtime Processes
use crate::rt::namespace::Namespace;
use crate::rt::subscription::{self, Subscription, SubscriptionHandle, SubscriptionId};
use crate::rt::{Error, Result};
use std::collections::HashMap;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinSet;
use tracing::debug;

/// Process set
pub(crate) type ProcessSet = JoinSet<ProcessResult>;

/// The end status of process
#[derive(Debug)]
pub struct ProcessResult {
    /// The ID the result is for
    pub proc_id: ProcessId,
}

/// Spawn a new process
pub(crate) fn spawn(id: ProcessId, proc_set: &mut ProcessSet) -> ProcessHandle {
    let (msg_tx, mut msg_rx) = mpsc::channel(32);
    let handle = ProcessHandle { id, msg_tx };
    let weak_handle = handle.clone().downgrade();
    proc_set.spawn(async move {
        let mut proc = Process::new(id, weak_handle);
        while let Some(msg) = msg_rx.recv().await {
            proc.handle_msg(msg).await;
            if proc.is_shutdown {
                break;
            }
        }
        ProcessResult { proc_id: id }
    });
    handle
}

/// ID assigned to [Process]
#[derive(Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub struct ProcessId(usize);
impl From<usize> for ProcessId {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

/// Handle to [Process]
#[derive(Debug, Clone)]
pub struct ProcessHandle {
    pub id: ProcessId,
    msg_tx: mpsc::Sender<Message>,
}

/// Weak version of [ProcessHandle]
#[derive(Debug, Clone)]
pub(crate) struct WeakProcessHandle {
    id: ProcessId,
    msg_tx: mpsc::WeakSender<Message>,
}

impl ProcessHandle {
    // TODO(security): Should proc handle allow raw calls into process's environment? Or more controlled messages from external?
    /// Send a blocking message to process, and get the result of evaluation
    pub(crate) async fn call(&self, form: lemma::Expr) -> Result<lemma::Expr> {
        let (tx, rx) = oneshot::channel();
        self.msg_tx.send(Message::Call(form, tx)).await?;
        rx.await
            .map_err(Error::FailedToReceiveResponseFromProcessTask)?
    }

    /// Send a message to add a subscription
    pub(crate) async fn add_subscription(&self, sub: subscription::Subscription) -> Result<()> {
        self.msg_tx.send(Message::AddSubscription(sub)).await?;
        Ok(())
    }

    /// Downgrade this proces handle into weak process handle that does not keep process alive
    pub(crate) fn downgrade(&self) -> WeakProcessHandle {
        WeakProcessHandle {
            id: self.id,
            msg_tx: self.msg_tx.downgrade(),
        }
    }

    /// Trigger graceful shutdown of process in next run of event loop
    pub(crate) async fn shutdown(&self) -> Result<()> {
        self.msg_tx.send(Message::Shutdown).await?;
        Ok(())
    }

    /// Check whether or not process is shutdown
    #[allow(dead_code)] // TODO Replace this? Only needed for testing
    pub(crate) async fn is_shutdown(&self) -> Result<bool> {
        let (tx, rx) = oneshot::channel();
        match self.msg_tx.send(Message::IsShutdown(tx)).await {
            Ok(()) => (),
            Err(_) => return Ok(true), // event loop is already shutdown
        }
        match rx.await {
            Ok(res) => Ok(res),
            Err(_) => Ok(true), // event loop is already shutdown
        }
    }
}

impl WeakProcessHandle {
    /// Upgrade into process handle
    pub(crate) fn upgrade(&self) -> Option<ProcessHandle> {
        self.msg_tx.upgrade().map(|msg_tx| ProcessHandle {
            id: self.id,
            msg_tx,
        })
    }
}

/// Messages that [Process] responds to
#[derive(Debug)]
pub enum Message {
    Call(lemma::Expr, oneshot::Sender<Result<lemma::Expr>>),
    AddSubscription(subscription::Subscription),
    Shutdown,
    IsShutdown(oneshot::Sender<bool>),
}

/// A process that runs within the runtime
pub(crate) struct Process {
    /// The unique id for process
    id: ProcessId,
    /// Whether or not process should exit in next cycle of event loop
    is_shutdown: bool,
    /// The weak process handle that this process may handoff to external tasks
    handle: WeakProcessHandle,
    /// The namespace for process
    ns: Namespace,
    /// Handles to subscriptions for this process
    subscriptions: HashMap<SubscriptionId, SubscriptionHandle>,
    /// The next subscription ID to assign
    next_sub_id: usize,
}

impl Process {
    pub(crate) fn new(id: ProcessId, handle: WeakProcessHandle) -> Self {
        debug!("proc {:?} created", id);
        Self {
            id,
            handle,
            ns: Namespace::new(),
            subscriptions: HashMap::new(),
            next_sub_id: 0,
            is_shutdown: false,
        }
    }

    pub(crate) async fn handle_msg(&mut self, msg: Message) {
        debug!("proc {:?} handle_msg - {:?}", self.id, msg);
        match msg {
            Message::Call(f, tx) => {
                let res = self.eval(&f);
                let _ = tx.send(res);
            }
            Message::AddSubscription(s) => self.add_subscription(s),
            Message::Shutdown => self.shutdown(),
            Message::IsShutdown(rx) => {
                let _ = rx.send(self.is_shutdown);
            }
        }
    }

    /// Evaluate given form in process's environment
    fn eval(&mut self, form: &lemma::Expr) -> Result<lemma::Expr> {
        self.ns.eval(form)
    }

    /// Add a new subscription to this process
    fn add_subscription(&mut self, sub: Subscription) {
        let id = SubscriptionId::from(self.next_sub_id);
        self.next_sub_id = self.next_sub_id.wrapping_add(1);
        self.subscriptions
            .insert(id, subscription::start(id, sub, self.handle.clone()));
    }

    /// Mark process for shutdown
    fn shutdown(&mut self) {
        if self.is_shutdown {
            return;
        }
        self.subscriptions.drain().for_each(|(_id, s)| s.abort());
        self.is_shutdown = true
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use lemma::parse as p;
//     use lemma::Form;
//     use tracing_test::traced_test;

//     #[tokio::test]
//     #[ignore] // TODO: Enable me
//     async fn proc_call() {
//         let mut proc_set = ProcessSet::new();
//         let proc = spawn(ProcessId::from(1), &mut proc_set);

//         assert!(matches!(
//             proc.call(p("(def echo (lambda (x) x))").unwrap())
//                 .await
//                 .unwrap(),
//             Form::Lambda(_)
//         ));

//         assert_eq!(
//             proc.call(p("(echo \"Hello world\")").unwrap())
//                 .await
//                 .unwrap(),
//             Form::string("Hello world")
//         );
//     }

//     #[tokio::test]
//     async fn proc_weak_handle_upgrade() {
//         let mut proc_set = ProcessSet::new();
//         let proc = spawn(ProcessId::from(1), &mut proc_set);
//         let weak = proc.downgrade();

//         assert!(
//             weak.upgrade().is_some(),
//             "Upgrading weak handle before original proc handle was dropped should return Some"
//         );
//     }

//     #[tokio::test]
//     async fn proc_weak_handle_upgrade_after_drop() {
//         let mut proc_set = ProcessSet::new();
//         let proc = spawn(ProcessId::from(1), &mut proc_set);
//         let weak = proc.downgrade();
//         drop(proc);

//         assert!(
//             weak.upgrade().is_none(),
//             "Upgrading weak handle after original proc handle was dropped should return None"
//         );
//     }

//     #[tokio::test]
//     #[traced_test]
//     async fn proc_shutdown_via_handle_shutdown() {
//         let mut proc_set = ProcessSet::new();
//         let proc = spawn(ProcessId::from(1), &mut proc_set);
//         assert!(!proc.is_shutdown().await.unwrap());
//         proc.shutdown().await.expect("Shutdown message should send");
//         assert!(proc.is_shutdown().await.unwrap());
//     }

//     // TODO: Test: that cast is nonblocking, even for long-running operations
//     // TODO: Test: that killing process is not blocked by long-running operations
//     // TODO: Test: Subscriptions are ignored / cancelled when process handle is dropped
//     // TODO: Test: Shutting down process aborts subscription
// }
