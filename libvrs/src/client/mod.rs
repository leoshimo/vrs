//! Headless client implementation for vrs runtime

mod error;

use std::collections::HashMap;

use crate::{
    connection::{Connection, Message, Request, Response, SubscriptionRequest, SubscriptionUpdate},
    rt::program::{Form, KeywordId},
};
use tokio::sync::{broadcast, mpsc, oneshot};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, warn};

use self::error::Error;

pub type Result<T> = std::result::Result<T, Error>;

/// Handle for vrs client
#[derive(Debug)]
pub struct Client {
    /// Sender half to send messages to shared task
    hdl_tx: mpsc::Sender<Event>,
    /// Cancellation token to shutdown async task
    cancel: CancellationToken,
}

/// Messages processed by event loop
#[derive(Debug)]
pub enum Event {
    /// Event for sending request to remote
    SendRequest {
        req: lyric::Form,
        tx: oneshot::Sender<Response>,
    },
    /// Event when receiving response from remote
    RecvResponse(Response),

    /// Subscribe command
    SubscribeRequest(KeywordId, oneshot::Sender<Result<Subscription>>),
    /// Subscription dropped
    SubscriptionDropped(KeywordId),

    /// Topic update for active subscription
    RecvSubscriptionUpdate(SubscriptionUpdate),
}

/// The state of active [Client]
#[derive(Debug)]
struct State {
    /// The connection to runtime
    conn: Connection,
    /// Next request id to use
    next_req_id: u32,
    /// Maps req_ids to Sender channel for responses
    inflight_reqs: HashMap<u32, oneshot::Sender<Response>>,
    /// Active subscriptions managed by client
    sub_txs: HashMap<KeywordId, ActiveSubscription>,
    /// Weak TX to event loop
    hdl_tx: mpsc::WeakSender<Event>,
}

/// Subscription record active on client
#[derive(Debug)]
struct ActiveSubscription {
    count: usize,
    sub_tx: broadcast::Sender<Form>,
}

/// An active subscription channel watched by client
#[derive(Debug)]
pub struct Subscription {
    topic: KeywordId,
    rx: broadcast::Receiver<Form>,
    on_drop_tx: Option<oneshot::Sender<()>>,
}

impl Subscription {
    /// Retrieve the topic
    pub fn topic(&self) -> &KeywordId {
        &self.topic
    }

    /// Receive next event of subscription
    pub async fn recv(&mut self) -> Result<Form> {
        Ok(self.rx.recv().await?)
    }
}

impl std::ops::Drop for Subscription {
    fn drop(&mut self) {
        if let Some(on_drop_tx) = self.on_drop_tx.take() {
            let _ = on_drop_tx.send(());
        }
    }
}

impl Client {
    /// Create new client from connection transport between client and runtime
    pub fn new(conn: Connection) -> Self {
        let (hdl_tx, hdl_rx) = mpsc::channel(32);
        let cancel = CancellationToken::new();

        // TODO: Move to dedicated client task runloop + state
        let cancel_clone = cancel.clone();
        let state = State::new(hdl_tx.downgrade(), conn);
        tokio::spawn(async move {
            tokio::select! {
                res = run(state, hdl_rx) => {
                    if let Err(e) = res {
                        eprintln!("Client terminated with err - {e}");
                    }
                },
                _ = cancel_clone.cancelled() => {
                    debug!("terminating client...");
                }
            }
        });
        Self { hdl_tx, cancel }
    }

    /// Dispatch a request
    pub async fn request(&self, req: lyric::Form) -> Result<Response> {
        debug!("request req = {}", req);
        let (resp_tx, resp_rx) = oneshot::channel();
        self.hdl_tx
            .send(Event::SendRequest { req, tx: resp_tx })
            .await?;
        Ok(resp_rx.await?)
    }

    /// Subscribe to a new topic
    pub async fn subscribe(&self, topic: KeywordId) -> Result<Subscription> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.hdl_tx
            .send(Event::SubscribeRequest(topic, resp_tx))
            .await?;
        resp_rx.await?
    }

    /// Detect if client has terminated
    pub async fn closed(&self) {
        self.hdl_tx.closed().await
    }

    /// Initiate Shutdown. The future completes when shutdown is complete
    pub async fn shutdown(&self) {
        debug!("shutdown - start");
        self.cancel.cancel();
        let _ = self.hdl_tx.closed().await; // wait until rx drop in `run`
        debug!("shutdown - done");
    }
}

/// Run client task over command channel and connection
async fn run(mut state: State, mut hdl_rx: mpsc::Receiver<Event>) -> Result<()> {
    loop {
        let ev = tokio::select! {
            Some(e) = hdl_rx.recv() => e,
            msg = state.conn.recv() => match msg {
                Some(msg) => msg.map(Event::try_from)??,
                None => return Err(Error::Disconnected),
            },
        };
        state.handle_event(ev).await?;
    }
}

impl State {
    fn new(hdl_tx: mpsc::WeakSender<Event>, conn: Connection) -> Self {
        Self {
            conn,
            next_req_id: 0,
            inflight_reqs: HashMap::new(),
            sub_txs: Default::default(),
            hdl_tx,
        }
    }

    async fn handle_event(&mut self, e: Event) -> Result<()> {
        debug!("handle_event e = {:?}", e);
        match e {
            Event::SendRequest {
                req: contents,
                tx: resp_tx,
            } => self.handle_request(contents, resp_tx).await,
            Event::RecvResponse(resp) => self.handle_recv_response(resp).await,
            Event::SubscribeRequest(topic, resp_tx) => {
                let res = self.handle_subscribe(topic).await;
                let _ = resp_tx.send(res);
                Ok(())
            }
            Event::RecvSubscriptionUpdate(update) => {
                self.handle_sub_update(update);
                Ok(())
            }
            Event::SubscriptionDropped(topic) => self.handle_sub_drop(topic).await,
        }
    }

    /// Handle a send request event
    async fn handle_request(
        &mut self,
        contents: lyric::Form,
        resp_tx: oneshot::Sender<Response>,
    ) -> Result<()> {
        let req = Request {
            id: self.next_req_id,
            contents,
        };
        self.next_req_id += 1;
        self.inflight_reqs.insert(req.id, resp_tx);
        Ok(self.conn.send(&Message::Request(req)).await?)
    }

    /// Handle a recv response event
    async fn handle_recv_response(&mut self, resp: Response) -> Result<()> {
        match self.inflight_reqs.remove(&resp.req_id) {
            Some(tx) => {
                let _ = tx.send(resp);
                Ok(())
            }
            None => Err(Error::Internal(format!(
                "Received unexpected response for request - {:?}",
                resp
            ))),
        }
    }

    /// Handle a subscription request
    async fn handle_subscribe(&mut self, topic: KeywordId) -> Result<Subscription> {
        let (on_drop_tx, on_drop_rx) = oneshot::channel();

        let hdl_tx = self.hdl_tx.clone();
        let topic_clone = topic.clone();
        tokio::spawn(async move {
            let _ = on_drop_rx.await;
            if let Some(hdl_tx) = hdl_tx.upgrade() {
                let _ = hdl_tx.send(Event::SubscriptionDropped(topic_clone)).await;
            }
        });

        match self.sub_txs.get_mut(&topic) {
            Some(record) => {
                record.count += 1;
                Ok(Subscription {
                    topic,
                    rx: record.sub_tx.subscribe(),
                    on_drop_tx: Some(on_drop_tx),
                })
            }
            None => {
                let (tx, rx) = broadcast::channel(32);
                self.sub_txs.insert(
                    topic.clone(),
                    ActiveSubscription {
                        count: 1,
                        sub_tx: tx,
                    },
                );
                self.conn
                    .send(&Message::SubscriptionStart(SubscriptionRequest {
                        topic: topic.clone(),
                    }))
                    .await?;
                Ok(Subscription {
                    topic,
                    rx,
                    on_drop_tx: Some(on_drop_tx),
                })
            }
        }
    }

    /// Handle topic updated over conn
    fn handle_sub_update(&self, update: SubscriptionUpdate) {
        match self.sub_txs.get(&update.topic) {
            Some(s) => {
                if let Err(e) = s.sub_tx.send(update.contents) {
                    error!("Error while sending update - {e}");
                }
            }
            None => {
                warn!(
                    "Received topic_updated for unsubscribed topic - {}",
                    update.topic
                );
            }
        }
    }

    async fn handle_sub_drop(&mut self, topic: KeywordId) -> Result<()> {
        let record = self
            .sub_txs
            .get_mut(&topic)
            .expect("Should only see drop events for active subscriptions");
        record.count -= 1;
        if record.count == 0 {
            self.sub_txs.remove(&topic);
            self.conn.send(&Message::SubscriptionEnd(topic)).await?;
        }

        Ok(())
    }
}

impl TryFrom<Message> for Event {
    type Error = Error;
    fn try_from(value: Message) -> Result<Self> {
        match value {
            Message::Response(resp) => Ok(Self::RecvResponse(resp)),
            Message::SubscriptionUpdate(update) => Ok(Self::RecvSubscriptionUpdate(update)),
            _ => Err(Error::Internal(format!(
                "Client unexpectedly received {:?}",
                value
            ))),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::connection::{Connection, SubscriptionRequest};
    use assert_matches::assert_matches;
    use lyric::Form;
    use tokio::task::yield_now;

    #[tokio::test]
    async fn request_response() {
        let (local, mut remote) = Connection::pair().unwrap();

        tokio::spawn(async move {
            loop {
                match remote.recv_req().await {
                    Some(Ok(req)) => {
                        let _ = remote
                            .send_resp(Response {
                                req_id: req.id,
                                contents: Ok(Form::string("response")),
                            })
                            .await;
                    }
                    _ => panic!("Unexpected recv on connection"),
                }
            }
        });

        let client = Client::new(local);
        let resp = client.request(Form::string("request")).await.unwrap();

        assert_matches!(
            resp,
            Response { contents: Ok(contents), .. } if contents == Form::string("response")
        );
    }

    #[tokio::test]
    async fn closed_after_remote_conn_drop() {
        use std::time::Duration;
        use tokio::time::timeout;

        let (local, remote) = Connection::pair().unwrap();
        let client = Client::new(local);

        let closed = client.closed();

        drop(remote); // drop remote conn

        timeout(Duration::from_millis(10), closed)
            .await
            .expect("client.closed() should complete when remote connection is dropped");
    }

    #[tokio::test]
    async fn request_errs_remote_conn_drop() {
        use std::time::Duration;
        use tokio::time::timeout;

        let (local, mut remote) = Connection::pair().unwrap();

        // Remote drops `remote` after first request
        tokio::spawn(async move {
            let _ = remote.recv().await;
            // remote is dropped w/o response
        });

        let client = Client::new(local);

        let req = client.request(lyric::Form::string("hi"));
        let resp = timeout(Duration::from_millis(10), req)
            .await
            .expect("Request should be notified that remote connection was dropped before timeout");

        assert_matches!(
            resp,
            Err(Error::FailedToRecv(_)),
            "Request should error when connection terminates"
        );
    }

    #[tokio::test]
    async fn subscribe() {
        let (local, mut remote) = Connection::pair().unwrap();
        let client = Client::new(local);

        let mut sub = client.subscribe(KeywordId::from("my_topic")).await.unwrap();
        assert_eq!(
            sub.topic(),
            &KeywordId::from("my_topic"),
            "subscription should have matching topic"
        );

        let msg = remote
            .recv()
            .await
            .expect("should receive message")
            .unwrap();
        assert_eq!(
            msg,
            Message::SubscriptionStart(SubscriptionRequest {
                topic: KeywordId::from("my_topic")
            }),
            "remote should receive corresponding subscribe request"
        );

        remote
            .send(&Message::SubscriptionUpdate(SubscriptionUpdate {
                topic: KeywordId::from("not_my_topic"),
                contents: Form::string("goodbye"),
            }))
            .await
            .unwrap();

        remote
            .send(&Message::SubscriptionUpdate(SubscriptionUpdate {
                topic: KeywordId::from("my_topic"),
                contents: Form::string("hello"),
            }))
            .await
            .unwrap();

        let res = sub.recv().await.unwrap();
        assert_eq!(res, Form::string("hello"));
    }

    #[tokio::test]
    async fn subscribe_multi() {
        let (local, mut remote) = Connection::pair().unwrap();
        let client = Client::new(local);

        let sub1 = client.subscribe(KeywordId::from("topic1")).await.unwrap();
        let sub2 = client.subscribe(KeywordId::from("topic2")).await.unwrap();

        let sub_task = |mut sub: Subscription| {
            tokio::spawn(async move {
                let mut forms = vec![];
                while let Ok(f) = sub.recv().await {
                    yield_now().await;
                    if f == Form::Nil {
                        break;
                    }
                    forms.push(f);
                    yield_now().await;
                }
                forms
            })
        };

        let sub1_task = sub_task(sub1);
        let sub2_task = sub_task(sub2);

        for contents in [
            Form::string("one"),
            Form::string("two"),
            Form::string("three"),
            Form::Nil,
        ] {
            remote
                .send(&Message::SubscriptionUpdate(SubscriptionUpdate {
                    topic: KeywordId::from("topic1"),
                    contents,
                }))
                .await
                .unwrap();
        }

        for contents in [Form::Int(4), Form::Int(5), Form::Int(6), Form::Nil] {
            remote
                .send(&Message::SubscriptionUpdate(SubscriptionUpdate {
                    topic: KeywordId::from("topic2"),
                    contents,
                }))
                .await
                .unwrap();
        }

        assert_eq!(
            sub1_task.await.unwrap(),
            vec![
                Form::string("one"),
                Form::string("two"),
                Form::string("three"),
            ],
            "should receive forms for topic in order"
        );
        assert_eq!(
            sub2_task.await.unwrap(),
            vec![Form::Int(4), Form::Int(5), Form::Int(6),]
        );
    }

    #[tokio::test]
    async fn subscription_drop_to_zero() {
        let (local, mut remote) = Connection::pair().unwrap();
        let client = Client::new(local);

        let remote_task = tokio::spawn(async move {
            let mut msgs = vec![];
            while msgs.len() < 2 {
                msgs.push(remote.recv().await.unwrap().unwrap());
            }
            msgs
        });

        let sub = client.subscribe(KeywordId::from("my_topic")).await.unwrap();
        drop(sub); // should send SubscriptionEnd

        let remote_msgs = remote_task.await.unwrap();

        assert_eq!(
            remote_msgs,
            vec![
                Message::SubscriptionStart(SubscriptionRequest {
                    topic: KeywordId::from("my_topic")
                }),
                Message::SubscriptionEnd(KeywordId::from("my_topic")),
            ]
        );
    }

    #[tokio::test]
    async fn subscription_drop_multi() {
        let (local, mut remote) = Connection::pair().unwrap();
        let client = Client::new(local);

        let remote_task = tokio::spawn(async move {
            let mut msgs = vec![];
            while msgs.len() < 2 {
                msgs.push(remote.recv().await.unwrap().unwrap());
            }
            msgs
        });

        let sub = client.subscribe(KeywordId::from("my_topic")).await.unwrap();
        let sub2 = client.subscribe(KeywordId::from("my_topic")).await.unwrap();

        drop(sub);
        drop(sub2);

        let remote_msgs = remote_task.await.unwrap();

        assert_eq!(
            remote_msgs,
            vec![
                Message::SubscriptionStart(SubscriptionRequest {
                    topic: KeywordId::from("my_topic")
                }),
                Message::SubscriptionEnd(KeywordId::from("my_topic")),
            ]
        );
    }

    #[tokio::test]
    async fn subscription_drop_decrement() {
        let (local, mut remote) = Connection::pair().unwrap();
        let client = Client::new(local);

        let remote_task = tokio::spawn(async move {
            let mut msgs = vec![];
            while msgs.len() < 2 {
                msgs.push(remote.recv().await.unwrap().unwrap());
            }
            msgs
        });

        tokio::spawn(async move {
            let sub = client.subscribe(KeywordId::from("my_topic")).await.unwrap();
            let _sub2 = client.subscribe(KeywordId::from("my_topic")).await.unwrap();
            drop(sub); // Only drop sub1

            loop {
                let _ = client.request(Form::Nil).await; // "marker" API call
            }
        });

        let remote_msgs = remote_task.await.unwrap();

        assert_matches!(
            remote_msgs[..],
            [Message::SubscriptionStart(_), Message::Request(_)],
            "should not see SubscriptionEnd"
        );
    }
}
