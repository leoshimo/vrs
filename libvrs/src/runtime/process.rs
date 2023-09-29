//! Runtime Processes
use super::namespace::Namespace;
use super::v2::{Error, Result};
use tokio::sync::{mpsc, oneshot};

/// Spawn a new process
pub(crate) fn spawn() -> ProcessHandle {
    let (msg_tx, mut msg_rx) = mpsc::channel(32);
    tokio::spawn(async move {
        let mut proc = Process::new();
        while let Some(msg) = msg_rx.recv().await {
            proc.handle_msg(msg).await;
        }
        Ok::<(), Error>(())
    });

    ProcessHandle { msg_tx }
}

/// ID assigned to [Process]
#[derive(Debug, PartialEq, Clone, Copy)]
pub(crate) struct ProcessId(usize);

/// Handle to [Process]
#[derive(Debug)]
pub(crate) struct ProcessHandle {
    msg_tx: mpsc::Sender<Message>,
}

impl ProcessHandle {
    /// Evalate given expression in process
    pub(crate) async fn eval_form(&self, form: lemma::Form) -> Result<lemma::Form> {
        let (tx, rx) = oneshot::channel();
        self.msg_tx.send(Message::EvaluateForm(form, tx)).await?;
        rx.await?
    }
}

pub enum Message {
    EvaluateForm(lemma::Form, oneshot::Sender<Result<lemma::Form>>),
}

/// A process that runs within the runtime
pub(crate) struct Process<'a> {
    ns: Namespace<'a>,
}

impl Process<'_> {
    pub(crate) fn new() -> Self {
        Self {
            ns: Namespace::new(),
        }
    }

    pub(crate) async fn handle_msg(&mut self, msg: Message) {
        match msg {
            Message::EvaluateForm(f, tx) => {
                let res = self.ns.eval(&f);
                let _ = tx.send(res);
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
    async fn proc_eval_form() {
        let proc = spawn();

        assert!(matches!(
            proc.eval_form(p("(def echo (lambda (x) x))").unwrap())
                .await
                .unwrap(),
            Form::Lambda(_)
        ));

        assert_eq!(
            proc.eval_form(p("(echo \"Hello world\")").unwrap())
                .await
                .unwrap(),
            Form::string("Hello world")
        );
    }
}
