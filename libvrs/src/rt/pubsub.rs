#![allow(dead_code)]
//! Future-only, best-effort pub/sub, locally and across direct peer links.
//!
//! Each local publication is offered to the peer manager through a bounded
//! stream. Peers inject received publications into their local broker without
//! relaying them. There is no replay on reconnect or transitive routing through
//! intermediate nodes. Only values supported by the peer wire format cross
//! nodes; other values retain local delivery and produce a transport warning.
// TODO: Leased Topics: Topics that can only be published by process that "claimed" that initially. On process exit, PubSub cleans Topic
// TODO: Namespaced Topics: Add topics to namespaces (?) e.g. global, process-specific, etc
// TODO: Think - Is PubSub general-case for Registry? I.e. Each process has special topic

use std::collections::HashMap;
use std::time::Instant;

use crate::{Error, Result, Val};
use lyric::KeywordId;
use tokio::sync::{
    broadcast::{self, error::RecvError},
    mpsc, oneshot,
};

use tracing::{info, warn};

/// Handle to spawned [PubSub] task
#[derive(Debug, Clone)]
pub(crate) struct PubSubHandle {
    tx: mpsc::Sender<Cmd>,
    local_publications: broadcast::Sender<Publication>,
}

/// A publication originating on this node, for best-effort peer fanout.
#[derive(Debug, Clone)]
pub(crate) struct Publication {
    pub topic: KeywordId,
    pub val: Val,
    pub published_at: Instant,
}

/// Node-local pubsub task, with an outbound stream for connected peers.
#[derive(Debug)]
pub(crate) struct PubSub {
    topics: HashMap<KeywordId, Topic>,
    local_publications: broadcast::Sender<Publication>,
}

/// Handle to active subscription.
/// Drop to unsubscribe
#[derive(Debug)]
pub(crate) struct Subscription {
    id: KeywordId,
    rx: broadcast::Receiver<Val>,
}

/// Internal data structure for managing active subscriptions
#[derive(Debug)]
struct Topic {
    id: KeywordId,
    tx: broadcast::Sender<Val>,
}

#[derive(Debug)]
enum Cmd {
    TryPublish {
        topic: KeywordId,
        val: Val,
    },
    Subscribe {
        topic: KeywordId,
        resp_tx: oneshot::Sender<Result<Subscription>>,
    },
    Publish {
        topic: KeywordId,
        val: Val,
        forward_to_peers: bool,
        resp_tx: oneshot::Sender<Result<()>>,
    },
    Clear {
        topic: KeywordId,
        resp_tx: oneshot::Sender<Result<()>>,
    },
}

impl PubSubHandle {
    pub(crate) fn local_publications(&self) -> broadcast::Receiver<Publication> {
        self.local_publications.subscribe()
    }

    /// Best-effort publication from synchronous observation hooks. Never wait
    /// for the broker or a subscriber; callers decide how to recover from loss.
    pub(crate) fn try_publish(&self, topic: &KeywordId, val: Val) -> bool {
        self.tx
            .try_send(Cmd::TryPublish {
                topic: topic.clone(),
                val,
            })
            .is_ok()
    }

    /// Establish a subscription for given handle
    pub(crate) async fn subscribe(&self, topic: &KeywordId) -> Result<Subscription> {
        info!("subscribe {topic}");
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
        info!("publish {topic} {val}");
        self.publish_inner(topic, val, true).await
    }

    /// Inject a peer publication locally without echoing it back or relaying it.
    pub(crate) async fn publish_from_peer(&self, topic: &KeywordId, val: Val) -> Result<()> {
        self.publish_inner(topic, val, false).await
    }

    async fn publish_inner(
        &self,
        topic: &KeywordId,
        val: Val,
        forward_to_peers: bool,
    ) -> Result<()> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx
            .send(Cmd::Publish {
                topic: topic.clone(),
                val,
                forward_to_peers,
                resp_tx,
            })
            .await
            .map_err(|_| Error::DeadPubSub)?;
        resp_rx.await?
    }

    /// Clear topic
    pub(crate) async fn clear(&self, topic: &KeywordId) -> Result<()> {
        info!("clear {topic}");
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

impl std::cmp::PartialEq for PubSubHandle {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(&self.tx, &other.tx)
    }
}

impl PubSub {
    /// Spawn a new global pubsub task
    pub(crate) fn spawn() -> PubSubHandle {
        let (tx, mut rx) = mpsc::channel(128);
        let (local_publications, _) = broadcast::channel(128);
        let mut pubsub = PubSub {
            topics: HashMap::new(),
            local_publications: local_publications.clone(),
        };

        tokio::spawn(async move {
            while let Some(cmd) = rx.recv().await {
                match cmd {
                    Cmd::TryPublish { topic, val } => {
                        // Optional observation logging happens on the broker,
                        // never inside the synchronous VM hook.
                        tracing::debug!(target: "vrs::observations", %topic, %val);
                        let _ = pubsub.handle_publish(topic, val, true);
                    }
                    Cmd::Subscribe { topic, resp_tx } => {
                        let res = pubsub.handle_subscribe(topic);
                        let _ = resp_tx.send(res);
                    }
                    Cmd::Publish {
                        topic,
                        val,
                        forward_to_peers,
                        resp_tx,
                    } => {
                        let res = pubsub.handle_publish(topic, val, forward_to_peers);
                        let _ = resp_tx.send(res);
                    }
                    Cmd::Clear { topic, resp_tx } => {
                        let res = pubsub.handle_clear(topic);
                        let _ = resp_tx.send(res);
                    }
                }
            }
        });
        PubSubHandle {
            tx,
            local_publications,
        }
    }

    /// Handle a new add subscription to add
    fn handle_subscribe(&mut self, topic_id: KeywordId) -> Result<Subscription> {
        let topic = self.get_topic(&topic_id);
        let sub = Subscription {
            id: topic_id,
            rx: topic.tx.subscribe(),
        };
        Ok(sub)
    }

    /// Publish new value on given topic
    fn handle_publish(
        &mut self,
        topic_id: KeywordId,
        val: Val,
        forward_to_peers: bool,
    ) -> Result<()> {
        // Publications on topics with no local subscribers need no topic state.
        if let Some(topic) = self.topics.get(&topic_id) {
            let _ = topic.tx.send(val.clone());
        }
        if forward_to_peers {
            // Never block local delivery on a slow or disconnected peer.
            let _ = self.local_publications.send(Publication {
                topic: topic_id,
                val,
                published_at: Instant::now(),
            });
        }
        Ok(())
    }

    /// Handle a clear request
    fn handle_clear(&mut self, topic_id: KeywordId) -> Result<()> {
        if self.topics.remove(&topic_id).is_none() {
            warn!("clearing unknown topic: {topic_id}");
        }
        Ok(())
    }

    /// Retrieve matching [Topic], or create a new one for topic id
    fn get_topic(&mut self, id: &KeywordId) -> &Topic {
        if !self.topics.contains_key(id) {
            let (tx, _) = broadcast::channel(32);
            let topic = Topic { id: id.clone(), tx };
            self.topics.insert(id.clone(), topic);
        }
        self.topics.get(id).expect("should contain key")
    }
}

impl Subscription {
    /// Future that completes when a new event is received for subscription
    pub(crate) async fn recv(&mut self) -> Option<Val> {
        loop {
            match self.rx.recv().await {
                Ok(v) => return Some(v),
                Err(RecvError::Lagged(_)) => {
                    warn!("Lagged on topic id = {}", self.id);
                    continue;
                }
                Err(RecvError::Closed) => return None,
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use std::time::Duration;
    use tokio::time::timeout;

    #[tokio::test]
    async fn peer_ingress_is_local_only_and_local_publications_leave_the_node() {
        let ps = PubSub::spawn();
        let topic = KeywordId::from("topic");
        let mut outbound = ps.local_publications();
        let mut sub = ps.subscribe(&topic).await.unwrap();

        ps.publish_from_peer(&topic, Val::Int(1)).await.unwrap();
        assert_eq!(sub.recv().await, Some(Val::Int(1)));
        assert!(matches!(
            outbound.try_recv(),
            Err(broadcast::error::TryRecvError::Empty)
        ));

        assert!(ps.try_publish(&topic, Val::Int(2)));
        let publication = timeout(Duration::from_secs(1), outbound.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(publication.topic, topic);
        assert_eq!(publication.val, Val::Int(2));
        assert_eq!(sub.recv().await, Some(Val::Int(2)));

        // Remote delivery must not depend on a local subscriber existing.
        drop(sub);
        ps.publish(&topic, Val::Int(3)).await.unwrap();
        assert_eq!(outbound.recv().await.unwrap().val, Val::Int(3));
    }

    #[tokio::test]
    async fn subscribe_then_publish() {
        let ps = PubSub::spawn();
        let topic = KeywordId::from("topic");
        let mut sub = ps.subscribe(&topic).await.unwrap();
        ps.publish(&topic, Val::string("hi")).await.unwrap();
        assert_eq!(sub.recv().await.unwrap(), Val::string("hi"))
    }

    #[tokio::test]
    async fn publish_then_subscribe() {
        let ps = PubSub::spawn();
        let topic = KeywordId::from("topic");

        ps.publish(&topic, Val::string("hi")).await.unwrap();
        let mut sub = ps.subscribe(&topic).await.unwrap();

        timeout(Duration::from_millis(0), sub.recv())
            .await
            .expect_err("Subscription should not see value before subscribe");
    }

    #[tokio::test]
    async fn subscription_captures_history() {
        let ps = PubSub::spawn();
        let topic = KeywordId::from("topic");

        ps.publish(&topic, Val::Int(0)).await.unwrap();

        let mut sub1 = ps.subscribe(&topic).await.unwrap(); // 1st sub

        ps.publish(&topic, Val::Int(1)).await.unwrap();
        ps.publish(&topic, Val::Int(2)).await.unwrap();

        let mut sub2 = ps.subscribe(&topic).await.unwrap(); // 2nd sub

        ps.publish(&topic, Val::Int(3)).await.unwrap();

        assert_eq!(
            (
                sub1.recv().await.unwrap(),
                sub1.recv().await.unwrap(),
                sub1.recv().await.unwrap(),
            ),
            (Val::Int(1), Val::Int(2), Val::Int(3)),
            "sub1 should receive 3 values after subscribing",
        );

        assert_eq!(
            sub2.recv().await.unwrap(),
            Val::Int(3),
            "sub2 should only see last value after subscribing",
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

        assert_eq!(strings.recv().await.unwrap(), Val::string("one"));
        assert_eq!(strings.recv().await.unwrap(), Val::string("two"));
        assert_eq!(strings.recv().await.unwrap(), Val::string("three"));

        assert_eq!(numbers.recv().await.unwrap(), Val::Int(1));
        assert_eq!(numbers.recv().await.unwrap(), Val::Int(2));
        assert_eq!(numbers.recv().await.unwrap(), Val::Int(3));
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
                while let Some(v) = sub.recv().await {
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
                while let Some(v) = sub.recv().await {
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
                while let Some(v) = sub.recv().await {
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
