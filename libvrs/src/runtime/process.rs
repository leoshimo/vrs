#![allow(dead_code)] // TODO: Remove me

//! Runtime Processes
use super::subscription::{self, Subscription, SubscriptionHandle, SubscriptionId};
use super::v2::{Error, Result};
use std::collections::HashMap;
use tokio::sync::{mpsc, oneshot};
use tracing::error;

/// Spawn a new process
pub(crate) fn spawn() -> ProcessHandle {
    spawn_with_sub(None)
}

// TODO: Add builder API
/// Spawn a process w/ given subscription
pub(crate) fn spawn_with_sub(sub: Option<Subscription>) -> ProcessHandle {
    let (msg_tx, mut msg_rx) = mpsc::channel(32);
    let handle = ProcessHandle { msg_tx };
    let weak_handle = handle.clone().downgrade();
    tokio::spawn(async move {
        let mut proc = Process::new(weak_handle);
        if let Some(sub) = sub {
            proc.add_subscription(sub);
        }
        while let Some(msg) = msg_rx.recv().await {
            proc.handle_msg(msg).await;
        }
        Ok::<(), Error>(())
    });
    handle
}

/// ID assigned to [Process]
#[derive(Debug, PartialEq, Clone, Copy)]
pub(crate) struct ProcessId(usize);

/// Handle to [Process]
#[derive(Debug, Clone)]
pub(crate) struct ProcessHandle {
    msg_tx: mpsc::Sender<Message>,
}

/// Weak version of [ProcessHandle]
#[derive(Debug, Clone)]
pub(crate) struct WeakProcessHandle {
    msg_tx: mpsc::WeakSender<Message>,
}

impl ProcessHandle {
    /// Send a blocking message to process, and get the result of evaluation
    pub(crate) async fn call(&self, form: lemma::Form) -> Result<lemma::Form> {
        let (tx, rx) = oneshot::channel();
        self.msg_tx.send(Message::Call(form, tx)).await?;
        rx.await?
    }

    /// Send a nonblocking message to process
    pub(crate) async fn cast(&self, form: lemma::Form) -> Result<()> {
        self.msg_tx.send(Message::Cast(form)).await?;
        Ok(())
    }

    /// Downgrade this proces handle into weak process handle that does not keep process alive
    pub(crate) fn downgrade(&self) -> WeakProcessHandle {
        WeakProcessHandle {
            msg_tx: self.msg_tx.downgrade(),
        }
    }
}

impl WeakProcessHandle {
    /// Upgrade into process handle
    pub(crate) fn upgrade(&self) -> Option<ProcessHandle> {
        self.msg_tx.upgrade().map(|msg_tx| ProcessHandle { msg_tx })
    }
}

/// Messages that [Process] responds to
pub enum Message {
    Call(lemma::Form, oneshot::Sender<Result<lemma::Form>>),
    Cast(lemma::Form),
    AddSubscription(subscription::Subscription),
}

/// A process that runs within the runtime
pub(crate) struct Process<'a> {
    /// The weak process handle that this process may handoff to external tasks
    handle: WeakProcessHandle,
    /// Environment of interpreter
    env: lemma::Env<'a>,
    /// Handles to subscriptions for this process
    subscriptions: HashMap<SubscriptionId, SubscriptionHandle>,
    /// The next subscription ID to assign
    next_sub_id: usize,
}

impl Process<'_> {
    pub(crate) fn new(handle: WeakProcessHandle) -> Self {
        Self {
            handle,
            env: lemma::lang::std_env(),
            subscriptions: HashMap::new(),
            next_sub_id: 0,
        }
    }

    pub(crate) async fn handle_msg(&mut self, msg: Message) {
        match msg {
            Message::Call(f, tx) => {
                let res = self.eval(&f);
                let _ = tx.send(res);
            }
            Message::Cast(f) => {
                let _ = self.eval(&f);
            }
            Message::AddSubscription(s) => self.add_subscription(s),
        }
    }

    /// Evaluate given form in process's environment
    fn eval(&mut self, form: &lemma::Form) -> Result<lemma::Form> {
        Ok(lemma::eval(form, &mut self.env)?)
    }

    /// Add a new subscription to this process
    fn add_subscription(&mut self, sub: Subscription) {
        let id = SubscriptionId::from(self.next_sub_id);
        self.next_sub_id = self.next_sub_id.wrapping_add(1);
        self.subscriptions
            .insert(id, subscription::start(id, sub, self.handle.clone()));
    }

    /// Remove a subscription from this process
    fn remove_subscription(&mut self, id: SubscriptionId) {
        match self.subscriptions.remove(&id) {
            Some(sub) => sub.abort(),
            None => {
                // TODO - Report errors?
                error!("No subscription found for subscription id {id}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lemma::parse as p;
    use lemma::Form;

    #[tokio::test]
    async fn proc_call() {
        let proc = spawn();

        assert!(matches!(
            proc.call(p("(def echo (lambda (x) x))").unwrap())
                .await
                .unwrap(),
            Form::Lambda(_)
        ));

        assert_eq!(
            proc.call(p("(echo \"Hello world\")").unwrap())
                .await
                .unwrap(),
            Form::string("Hello world")
        );
    }

    #[tokio::test]
    async fn proc_cast() {
        let proc = spawn();

        proc.call(p("(def inc (lambda (x) (+ x 1)))").unwrap())
            .await
            .unwrap();

        assert!(matches!(
            proc.cast(p("(def count 0)").unwrap()).await,
            Ok(())
        ));
        assert!(matches!(
            proc.cast(p("(def count (inc count))").unwrap()).await,
            Ok(())
        ));
    }

    #[tokio::test]
    async fn proc_weak_handle_upgrade() {
        let proc = spawn();
        let weak = proc.downgrade();

        assert!(
            weak.upgrade().is_some(),
            "Upgrading weak handle before original proc handle was dropped should return Some"
        );
    }

    #[tokio::test]
    async fn proc_weak_handle_upgrade_after_drop() {
        let proc = spawn();
        let weak = proc.downgrade();
        drop(proc);

        assert!(
            weak.upgrade().is_none(),
            "Upgrading weak handle after original proc handle was dropped should return None"
        );
    }

    // TODO: Test: that cast is nonblocking, even for long-running operations
    // TODO: Test: that killing process is not blocked by long-running operations
    // TODO: Test: Subscriptions are ignored / cancelled when process handle is dropped
}
