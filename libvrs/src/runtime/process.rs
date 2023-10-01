#![allow(dead_code)] // TODO: Remove me

//! Runtime Processes
use super::v2::{Error, Result};
use tokio::sync::{mpsc, oneshot};

/// Spawn a new process
pub(crate) fn spawn() -> ProcessHandle {
    let (msg_tx, mut msg_rx) = mpsc::channel(32);
    let handle = ProcessHandle { msg_tx };
    let weak_handle = handle.clone().downgrade();
    tokio::spawn(async move {
        let mut proc = Process::new(weak_handle);
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
        self.msg_tx
            .upgrade()
            .and_then(|msg_tx| Some(ProcessHandle { msg_tx }))
    }
}

/// Messages that [Process] responds to
pub enum Message {
    Call(lemma::Form, oneshot::Sender<Result<lemma::Form>>),
    Cast(lemma::Form),
}

/// A process that runs within the runtime
pub(crate) struct Process<'a> {
    /// Environment of interpreter
    env: lemma::Env<'a>,
    /// The weak process handle that this process may handoff to external tasks
    handle: WeakProcessHandle,
}

impl Process<'_> {
    pub(crate) fn new(handle: WeakProcessHandle) -> Self {
        Self {
            env: lemma::lang::std_env(),
            handle,
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
        }
    }

    /// Evaluate given form in process's environment
    fn eval(&mut self, form: &lemma::Form) -> Result<lemma::Form> {
        Ok(lemma::eval(form, &mut self.env)?)
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
