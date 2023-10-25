//! A Process's Mailbox
use super::proc::{ProcessId, Val};
use crate::rt::{Error, Result};
use tokio::sync::{mpsc, oneshot};
use tracing::debug;

/// Handle to mailbox
#[derive(Debug, Clone)]
pub(crate) struct MailboxHandle {
    tx: mpsc::Sender<Cmd>,
}

/// Mailbox for given process
#[derive(Debug, Default)]
pub(crate) struct Mailbox {
    messages: Vec<Message>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Message {
    pub(crate) msg: Val,
}

/// Commands between mailbox handle and async task
#[derive(Debug)]
enum Cmd {
    OnReceive(Message),
    GetMessages(oneshot::Sender<Vec<Message>>),
}

impl MailboxHandle {
    /// Notify mailbox of new message
    pub(crate) async fn received(&self, msg: Message) -> Result<()> {
        self.tx
            .send(Cmd::OnReceive(msg))
            .await
            .map_err(|_| Error::NoMailbox)?;
        Ok(())
    }

    /// Request messages in mailbox
    pub(crate) async fn messages(&self) -> Result<Vec<Message>> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Cmd::GetMessages(tx))
            .await
            .map_err(|_| Error::NoMailbox)?;
        Ok(rx.await?)
    }
}

impl Mailbox {
    /// Spawn a new mailbox task
    pub(crate) fn spawn(id: ProcessId) -> MailboxHandle {
        let (tx, mut rx) = mpsc::channel(32);
        tokio::spawn(async move {
            let mut mailbox = Mailbox::default();
            while let Some(cmd) = rx.recv().await {
                debug!("mailbox {}: {:?}", id, cmd);
                match cmd {
                    Cmd::OnReceive(msg) => mailbox.on_receive(msg),
                    Cmd::GetMessages(tx) => {
                        let _ = tx.send(mailbox.messages.clone());
                    }
                }
            }
        });
        MailboxHandle { tx }
    }

    fn on_receive(&mut self, msg: Message) {
        self.messages.push(msg);
        // TODO: Implement resolving pending requests if any
    }
}

impl Message {
    pub(crate) fn new(_src: ProcessId, msg: Val) -> Self {
        Self { msg }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[tokio::test]
    async fn messages() {
        let mb = Mailbox::spawn(0);
        assert_eq!(mb.messages().await.unwrap(), vec![]);
    }

    #[tokio::test]
    async fn received() {
        let mb = Mailbox::spawn(0);

        mb.received(Message::new(1, Val::symbol("one")))
            .await
            .expect("Mailbox should receive msg");
        mb.received(Message::new(2, Val::symbol("two")))
            .await
            .expect("Mailbox should receive msg");
        mb.received(Message::new(3, Val::symbol("three")))
            .await
            .expect("Mailbox should receive msg");

        assert_eq!(
            mb.messages().await.unwrap(),
            vec![
                Message::new(1, Val::symbol("one")),
                Message::new(2, Val::symbol("two")),
                Message::new(3, Val::symbol("three")),
            ],
            "Messages should be present in order it was received"
        );
    }

    // TODO: Test messages can be received
    // TODO: Test that mailbox errors terminate process
}
