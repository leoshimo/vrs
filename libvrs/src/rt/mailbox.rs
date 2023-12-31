//! A Process's Mailbox
use std::collections::VecDeque;

use super::proc::ProcessId;
use super::program::{Pattern, Val};
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
    pending: Option<PendingPoll>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Message {
    pub(crate) contents: Val,
}

/// Commands between mailbox handle and async task
#[derive(Debug)]
enum Cmd {
    Push(Message),
    GetAll(oneshot::Sender<Vec<Message>>),
    Poll(Option<Pattern>, oneshot::Sender<Message>),
}

/// A pending handle for polling mailbox
#[derive(Debug)]
struct PendingPoll {
    pattern: Option<Pattern>,
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
    /// Blocks calling task until message is receive
    pub(crate) async fn poll(&self, pat: Option<Pattern>) -> Result<Message> {
        let (tx, rx) = oneshot::channel();
        self.tx
            .send(Cmd::Poll(pat, tx))
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
                    Cmd::Poll(pat, tx) => mailbox.handle_poll(pat, tx),
                }
            }
        });
        MailboxHandle { tx }
    }

    /// Push a new message into mailbox. This may resolve pending requests
    fn push(&mut self, msg: Message) {
        let fufills_pending = match &self.pending {
            Some(pending) => match &pending.pattern {
                Some(pat) => pat.is_match(&msg.contents),
                None => true,
            },
            None => false,
        };

        if fufills_pending {
            let pending = self.pending.take().unwrap();
            let _ = pending.tx.send(msg);
        } else {
            self.messages.push_back(msg);
        }
    }

    /// Handle a poll for message matching given [Pattern] that will be sent to `tx`
    fn handle_poll(&mut self, pattern: Option<Pattern>, tx: oneshot::Sender<Message>) {
        match self.pop_match(&pattern) {
            Some(msg) => {
                let _ = tx.send(msg);
            }
            None => {
                if self.pending.is_some() {
                    panic!("Unexpected poll on mailbox - mailbox should only be polled from process task");
                }
                self.pending = Some(PendingPoll { pattern, tx });
            }
        }
    }

    /// Find the first matching message for given pattern, if any
    fn pop_match(&mut self, pat: &Option<Pattern>) -> Option<Message> {
        match pat {
            None => self.messages.pop_front(),
            Some(pat) => {
                for i in 0..self.messages.len() {
                    if pat.is_match(&self.messages[i].contents) {
                        return self.messages.remove(i);
                    }
                }
                None
            }
        }
    }
}

impl Message {
    pub(crate) fn new(_src: ProcessId, msg: Val) -> Self {
        Self { contents: msg }
    }
}

#[cfg(test)]
mod tests {

    use lyric::parse;
    use tokio::task::yield_now;

    use super::*;

    #[tokio::test]
    async fn messages() {
        let mb = Mailbox::spawn(ProcessId::from(0));
        assert_eq!(mb.all().await.unwrap(), vec![]);
    }

    #[tokio::test]
    async fn all() {
        let mb = Mailbox::spawn(0.into());

        mb.push(Message::new(1.into(), Val::symbol("one")))
            .await
            .expect("Mailbox should receive msg");
        mb.push(Message::new(2.into(), Val::symbol("two")))
            .await
            .expect("Mailbox should receive msg");
        mb.push(Message::new(3.into(), Val::symbol("three")))
            .await
            .expect("Mailbox should receive msg");

        assert_eq!(
            mb.all().await.unwrap(),
            vec![
                Message::new(1.into(), Val::symbol("one")),
                Message::new(2.into(), Val::symbol("two")),
                Message::new(3.into(), Val::symbol("three")),
            ],
            "Messages should be present in order it was received"
        );
    }

    #[tokio::test]
    async fn poll_after_push() {
        let mb = Mailbox::spawn(0.into());

        mb.push(Message::new(
            1.into(),
            parse("(:hello \"one\" 2 :three)").unwrap().into(),
        ))
        .await
        .unwrap();

        // poll after push
        assert_eq!(
            mb.poll(None).await.unwrap(),
            Message::new(1.into(), parse("(:hello \"one\" 2 :three)").unwrap().into())
        );
    }

    #[tokio::test]
    async fn poll_before_push() {
        let mb = Mailbox::spawn(0.into());

        let mb_clone = mb.clone();
        let hdl = tokio::spawn(async move { mb_clone.poll(None).await });

        yield_now().await; // yield on current task to let poll run
        assert!(!hdl.is_finished(), "Task should block on poll");

        mb.push(Message::new(1.into(), Val::symbol("hi")))
            .await
            .unwrap();

        assert_eq!(
            hdl.await.unwrap().unwrap(),
            Message::new(1.into(), Val::symbol("hi")),
            "Poll should return with result"
        );
    }

    #[tokio::test]
    async fn poll_after_push_pattern() {
        let mb = Mailbox::spawn(0.into());

        let msg1 = Message::new(1.into(), Val::from_expr("(:one 1)").unwrap());
        let msg2 = Message::new(2.into(), Val::from_expr("(:two 2)").unwrap());
        let msg3 = Message::new(3.into(), Val::from_expr("(:three 3)").unwrap());
        let msg4 = Message::new(3.into(), Val::from_expr("(:four 4)").unwrap());

        mb.push(msg1.clone()).await.unwrap();
        mb.push(msg2.clone()).await.unwrap();
        mb.push(msg3.clone()).await.unwrap();
        mb.push(msg4.clone()).await.unwrap();

        let pat_two = Pattern::from_expr("(:two _)").unwrap();
        assert_eq!(
            mb.poll(Some(pat_two)).await.unwrap(),
            msg2,
            "Should return with matching message"
        );

        assert_eq!(
            mb.all().await.unwrap(),
            vec![msg1.clone(), msg3.clone(), msg4.clone()],
            "Should still have unmatched messages"
        );

        assert_eq!(
            mb.poll(Some(Pattern::from_expr("(a b)").unwrap()))
                .await
                .unwrap(),
            msg1,
            "If multiple messages match, earlier messages one should be returned first"
        );

        assert_eq!(
            mb.all().await.unwrap(),
            vec![msg3, msg4],
            "Should still have unmatched messages"
        );
    }

    #[tokio::test]
    async fn poll_before_push_pattern() {
        let mb = Mailbox::spawn(0.into());

        let mbc = mb.clone();
        let hdl = tokio::spawn(async move {
            vec![
                mbc.poll(Some(Pattern::from_expr("(:four _)").unwrap()))
                    .await
                    .unwrap(),
                mbc.poll(Some(Pattern::from_expr("(a b)").unwrap()))
                    .await
                    .unwrap(),
                mbc.poll(Some(Pattern::from_expr("(a b c)").unwrap()))
                    .await
                    .unwrap(),
            ]
        });

        yield_now().await;
        assert!(!hdl.is_finished(), "mb.poll task should not be finished");

        // Message sequences
        let msg1 = Message::new(1.into(), Val::from_expr("(:one 1)").unwrap()); // matches 2nd poll
        let msg2 = Message::new(2.into(), Val::from_expr("(:two 2 2)").unwrap()); // matches 3rd poll
        let msg3 = Message::new(3.into(), Val::from_expr("(:three 3)").unwrap()); // ignored
        let msg4 = Message::new(4.into(), Val::from_expr("(:four 4)").unwrap()); // matches 1st poll
        let msg5 = Message::new(5.into(), Val::from_expr("(:five 5 5)").unwrap()); // ignored
        mb.push(msg1.clone()).await.unwrap();
        mb.push(msg2.clone()).await.unwrap();
        mb.push(msg3.clone()).await.unwrap();
        mb.push(msg4.clone()).await.unwrap();
        mb.push(msg5.clone()).await.unwrap();

        assert_eq!(hdl.await.unwrap(), vec![msg4, msg1, msg2]);
        assert_eq!(mb.all().await.unwrap(), vec![msg3, msg5]);
    }
}
