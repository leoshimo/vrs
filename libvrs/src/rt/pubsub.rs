#![allow(dead_code)]
//! Global PubSub
// TODO: Leased Topics: Topics that can only be published by process that "claimed" that initially. On process exit, PubSub cleans Topic
// TODO: Namespaced Topics: Add topics to namespaces (?) e.g. global, process-specific, etc
// TODO: Think - Is PubSub general-case for Registry? I.e. Each process has special topic

use std::collections::HashMap;

use crate::{Error, Result, Val};
use lyric::KeywordId;
use tokio::sync::{mpsc, oneshot, watch};

/// Handle to spawned [PubSub] task
#[derive(Debug, Clone)]
pub(crate) struct PubSubHandle {
    tx: mpsc::Sender<Cmd>,
}

/// Global pubsub task
#[derive(Debug, Default)]
pub(crate) struct PubSub {
    topics: HashMap<KeywordId, Topic>,
}

/// Handle to active subscription.
/// Drop to unsubscribe
#[derive(Debug)]
pub(crate) struct Subscription {
    rx: watch::Receiver<Val>,
}

/// Internal data structure for managing active subscriptions
#[derive(Debug)]
struct Topic {
    id: KeywordId,
    tx: watch::Sender<Val>,
}

#[derive(Debug)]
enum Cmd {
    Subscribe {
        topic: KeywordId,
        resp_tx: oneshot::Sender<Result<Subscription>>,
    },
    Publish {
        topic: KeywordId,
        val: Val,
        resp_tx: oneshot::Sender<Result<()>>,
    },
    Clear {
        topic: KeywordId,
        resp_tx: oneshot::Sender<Result<()>>,
    },
}

impl PubSubHandle {
    /// Establish a subscription for given handle
    pub(crate) async fn subscribe(&self, topic: &KeywordId) -> Result<Subscription> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx
            .send(Cmd::Subscribe {
                topic: topic.clone(),
                resp_tx,
            })
            .await
            .map_err(|_| Error::DeadPubSub)?;
        resp_rx.await?
    }

    /// Publish a new value for given handle
    pub(crate) async fn publish(&self, topic: &KeywordId, val: Val) -> Result<()> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx
            .send(Cmd::Publish {
                topic: topic.clone(),
                val,
                resp_tx,
            })
            .await
            .map_err(|_| Error::DeadPubSub)?;
        resp_rx.await?
    }

    /// Clear topic
    pub(crate) async fn clear(&self, topic: &KeywordId) -> Result<()> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx
            .send(Cmd::Clear {
                topic: topic.clone(),
                resp_tx,
            })
            .await
            .map_err(|_| Error::DeadPubSub)?;
        resp_rx.await?
    }
}

impl PubSub {
    /// Spawn a new global pubsub task
    pub(crate) fn spawn() -> PubSubHandle {
        let (tx, mut rx) = mpsc::channel(128);

        tokio::spawn(async move {
            let mut pubsub = PubSub::default();
            while let Some(cmd) = rx.recv().await {
                match cmd {
                    Cmd::Subscribe { topic, resp_tx } => {
                        let res = pubsub.handle_subscribe(topic);
                        let _ = resp_tx.send(res);
                    }
                    Cmd::Publish {
                        topic,
                        val,
                        resp_tx,
                    } => {
                        let res = pubsub.handle_publish(topic, val);
                        let _ = resp_tx.send(res);
                    }
                    Cmd::Clear { topic, resp_tx } => {
                        let res = pubsub.handle_clear(topic);
                        let _ = resp_tx.send(res);
                    }
                }
            }
        });
        PubSubHandle { tx }
    }

    /// Handle a new add subscription to add
    fn handle_subscribe(&mut self, topic_name: KeywordId) -> Result<Subscription> {
        let topic = self.get_topic(topic_name);
        let sub = Subscription {
            rx: topic.tx.subscribe(),
        };
        Ok(sub)
    }

    /// Publish new value on given topic
    fn handle_publish(&mut self, topic_id: KeywordId, val: Val) -> Result<()> {
        let topic = self.get_topic(topic_id);
        let _ = topic.tx.send(val);
        Ok(())
    }

    /// Handle a clear request
    fn handle_clear(&mut self, topic_id: KeywordId) -> Result<()> {
        self.topics.remove(&topic_id);
        Ok(())
    }

    /// Retrieve matching [Topic], or create a new one for topic id
    fn get_topic(&mut self, id: KeywordId) -> &Topic {
        if !self.topics.contains_key(&id) {
            let (tx, _) = watch::channel(Val::Nil);
            let topic = Topic { id: id.clone(), tx };
            self.topics.insert(id.clone(), topic);
        }

        self.topics.get(&id).expect("should contain key")
    }
}

impl Subscription {
    /// Future that completes when a new event is received for subscription
    pub(crate) async fn next(&mut self) -> Option<Val> {
        match self.rx.changed().await {
            Ok(_) => Some(self.rx.borrow_and_update().clone()),
            Err(_) => None, // changed() returns Err iff Sender is dropped
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use std::time::Duration;
    use tokio::time::timeout;

    #[tokio::test]
    async fn subscribe_then_publish() {
        let ps = PubSub::spawn();
        let topic = KeywordId::from("topic");
        let mut sub = ps.subscribe(&topic).await.unwrap();
        ps.publish(&topic, Val::string("hi")).await.unwrap();
        assert_eq!(sub.next().await.unwrap(), Val::string("hi"))
    }

    #[tokio::test]
    async fn publish_then_subscribe() {
        let ps = PubSub::spawn();
        let topic = KeywordId::from("topic");

        ps.publish(&topic, Val::string("hi")).await.unwrap();
        let mut sub = ps.subscribe(&topic).await.unwrap();

        timeout(Duration::from_millis(0), sub.next())
            .await
            .expect_err("Subscription should not see value before subscribe");
    }

    #[tokio::test]
    async fn next_only_sees_recent() {
        let ps = PubSub::spawn();
        let topic = KeywordId::from("topic");

        let mut sub = ps.subscribe(&topic).await.unwrap();
        ps.publish(&topic, Val::Int(1)).await.unwrap();
        ps.publish(&topic, Val::Int(2)).await.unwrap();
        ps.publish(&topic, Val::Int(3)).await.unwrap();

        assert_eq!(
            sub.next().await.unwrap(),
            Val::Int(3),
            "Subscription should only deliver most recent value if next was not polled"
        );
    }

    #[tokio::test]
    async fn publish_separate_topics() {
        let ps = PubSub::spawn();
        let num_topic = KeywordId::from("numbers");
        let str_topic = KeywordId::from("strings");

        let mut numbers = ps.subscribe(&num_topic).await.unwrap();
        let mut strings = ps.subscribe(&str_topic).await.unwrap();

        ps.publish(&num_topic, Val::Int(1)).await.unwrap();
        ps.publish(&str_topic, Val::string("one")).await.unwrap();
        ps.publish(&num_topic, Val::Int(2)).await.unwrap();
        ps.publish(&num_topic, Val::Int(3)).await.unwrap();
        ps.publish(&str_topic, Val::string("two")).await.unwrap();
        ps.publish(&str_topic, Val::string("three")).await.unwrap();

        assert_eq!(strings.next().await.unwrap(), Val::string("three"));
        assert_eq!(numbers.next().await.unwrap(), Val::Int(3));
    }

    #[tokio::test]
    async fn publish_multi_other_task() {
        let ps = PubSub::spawn();
        let topic = KeywordId::from("topic");

        ps.publish(&topic, Val::string("zero")).await.unwrap();

        let hdl = {
            let ps = ps.clone();
            let topic = topic.clone();
            let mut sub = ps.subscribe(&topic).await.unwrap(); // subscribe in current task, then move sub into task, or messages may be lost
            tokio::spawn(async move {
                let mut res = vec![];
                while let Some(v) = sub.next().await {
                    res.push(v);
                }
                res
            })
        };

        ps.publish(&topic, Val::string("one")).await.unwrap();
        ps.publish(&topic, Val::string("two")).await.unwrap();
        ps.publish(&topic, Val::string("three")).await.unwrap();
        ps.clear(&topic).await.unwrap();

        let res = hdl.await.unwrap();
        assert_eq!(
            res,
            vec![Val::string("one"), Val::string("two"), Val::string("three"),],
            "Subscription should receive all published data after subscribing"
        )
    }

    #[tokio::test]
    async fn publish_multi_multiple_consumers() {
        let ps = PubSub::spawn();
        let topic = KeywordId::from("topic");

        ps.publish(&topic, Val::string("zero")).await.unwrap();

        // consumer 1
        let hdl1 = {
            let ps = ps.clone();
            let topic = topic.clone();
            let mut sub = ps.subscribe(&topic).await.unwrap(); // must subscribe in current task
            tokio::spawn(async move {
                let mut res = vec![];
                while let Some(v) = sub.next().await {
                    res.push(v);
                }
                res
            })
        };

        ps.publish(&topic, Val::string("one")).await.unwrap();

        // consumer 2
        let hdl2 = {
            let ps = ps.clone();
            let topic = topic.clone();
            let mut sub = ps.subscribe(&topic).await.unwrap(); // must subscribe in current task
            tokio::spawn(async move {
                let mut res = vec![];
                while let Some(v) = sub.next().await {
                    res.push(v);
                }
                res
            })
        };

        ps.publish(&topic, Val::string("two")).await.unwrap();
        ps.publish(&topic, Val::string("three")).await.unwrap();
        ps.clear(&topic).await.unwrap();

        assert_eq!(
            hdl1.await.unwrap(),
            vec![Val::string("one"), Val::string("two"), Val::string("three"),],
            "hdl1 should receive all published after subscribing"
        );
        assert_eq!(
            hdl2.await.unwrap(),
            vec![Val::string("two"), Val::string("three"),],
            "hdl2 should receive all published after subscribing"
        )
    }
}
