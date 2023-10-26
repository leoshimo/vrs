//! A Process's Mailbox
use std::collections::VecDeque;

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
    messages: VecDeque<Message>,
    pending: VecDeque<PendingPoll>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Message {
    pub(crate) msg: Val,
}

/// Commands between mailbox handle and async task
#[derive(Debug)]
enum Cmd {
    Push(Message),
    GetAll(oneshot::Sender<Vec<Message>>),
    Poll(oneshot::Sender<Message>),
}

/// A pending handle for polling mailbox
#[derive(Debug)]
struct PendingPoll {
    tx: oneshot::Sender<Message>,
}

impl MailboxHandle {
    /// Push a new message to mailbox
    pub(crate) async fn push(&self, msg: Message) -> Result<()> {
        self.tx
            .send(Cmd::Push(msg))
            .await
            .map_err(|_| Error::NoMailbox)?;
        Ok(())
    }

    /// Get all messages from mailbox
    pub(crate) async fn all(&self) -> Result<Vec<Message>> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Cmd::GetAll(tx))
            .await
            .map_err(|_| Error::NoMailbox)?;
        Ok(rx.await?)
    }

    /// Poll mailbox for matching message.
    /// Blocks calling task until message is received
    pub(crate) async fn poll(&self) -> Result<Message> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Cmd::Poll(tx))
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
                    Cmd::Push(msg) => mailbox.push(msg),
                    Cmd::GetAll(tx) => {
                        let msgs = mailbox.messages.iter().cloned().collect();
                        let _ = tx.send(msgs);
                    }
                    Cmd::Poll(tx) => mailbox.handle_poll(tx),
                }
            }
        });
        MailboxHandle { tx }
    }

    /// Push a new message into mailbox. This may resolve pending requests
    fn push(&mut self, msg: Message) {
        // check pending first
        match self.pending.pop_front() {
            Some(pending) => {
                let _ = pending.tx.send(msg);
            }
            None => self.messages.push_back(msg),
        }
    }

    /// Dequeue message in FIFO order
    fn handle_poll(&mut self, tx: oneshot::Sender<Message>) {
        match self.messages.pop_front() {
            Some(msg) => {
                let _ = tx.send(msg);
            }
            None => self.pending.push_back(PendingPoll { tx }),
        }
    }
}

impl Message {
    pub(crate) fn new(_src: ProcessId, msg: Val) -> Self {
        Self { msg }
    }
}

#[cfg(test)]
mod tests {

    use lyric::parse;
    use tokio::task::yield_now;

    use super::*;

    #[tokio::test]
    async fn messages() {
        let mb = Mailbox::spawn(0);
        assert_eq!(mb.all().await.unwrap(), vec![]);
    }

    #[tokio::test]
    async fn received() {
        let mb = Mailbox::spawn(0);

        mb.push(Message::new(1, Val::symbol("one")))
            .await
            .expect("Mailbox should receive msg");
        mb.push(Message::new(2, Val::symbol("two")))
            .await
            .expect("Mailbox should receive msg");
        mb.push(Message::new(3, Val::symbol("three")))
            .await
            .expect("Mailbox should receive msg");

        assert_eq!(
            mb.all().await.unwrap(),
            vec![
                Message::new(1, Val::symbol("one")),
                Message::new(2, Val::symbol("two")),
                Message::new(3, Val::symbol("three")),
            ],
            "Messages should be present in order it was received"
        );
    }

    #[tokio::test]
    async fn poll_after_push() {
        let mb = Mailbox::spawn(0);

        mb.push(Message::new(
            1,
            parse("(:hello \"one\" 2 :three)").unwrap().into(),
        ))
        .await
        .unwrap();

        // poll after push
        assert_eq!(
            mb.poll().await.unwrap(),
            Message::new(1, parse("(:hello \"one\" 2 :three)").unwrap().into())
        );
    }

    #[tokio::test]
    async fn poll_before_push() {
        let mb = Mailbox::spawn(0);

        let mb_clone = mb.clone();
        let hdl = tokio::spawn(async move { mb_clone.poll().await });

        assert!(!hdl.is_finished(), "Task should block on poll");

        yield_now().await; // yield on current task to let 2nd poll run

        mb.push(Message::new(1, Val::symbol("hi"))).await.unwrap();

        assert_eq!(
            hdl.await.unwrap().unwrap(),
            Message::new(1, Val::symbol("hi")),
            "Poll should return with result"
        );
    }

    // TODO: Test that mailbox errors terminate process
}
