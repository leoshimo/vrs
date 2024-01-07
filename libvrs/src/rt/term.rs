//! Controlling Terminal for Processes

use std::collections::{HashMap, VecDeque};

use crate::connection::{Message, SubscriptionRequest, SubscriptionUpdate};
use crate::rt::{Error, Result};
use crate::{Connection, Request, Response};
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;
use tracing::{error, info};

use super::program::{Form, KeywordId};
use super::pubsub::PubSubHandle;

/// Controlling terminal managing a client connection
/// Acts as a pseudo-process, with a single attached process for expression evaluations
/// [Term] has its own state for subscriptions, separately from attached process.
#[derive(Debug)]
pub struct Term {
    tx: mpsc::WeakSender<Cmd>,
    rx: mpsc::Receiver<Cmd>,
    conn: Connection,
    req_queue: VecDeque<Request>,
    read_req_queue: VecDeque<oneshot::Sender<Request>>,
    pubsub: PubSubHandle,
    active_subs: HashMap<KeywordId, JoinHandle<()>>,
}

/// Handle to [Term]
#[derive(Debug, Clone)]
pub struct TermHandle {
    tx: mpsc::Sender<Cmd>,
}

#[derive(Debug)]
enum Cmd {
    ReadRequest(oneshot::Sender<Request>),
    SendResponse(Response),
    NotifySubscriptionUpdate(KeywordId, Form),
}

impl TermHandle {
    /// Read request from terminal
    pub(crate) async fn read_request(&self) -> Result<Request> {
        let (req_tx, req_rx) = oneshot::channel();
        self.tx
            .send(Cmd::ReadRequest(req_tx))
            .await
            .map_err(|e| Error::NoMessageReceiver(format!("read_request failed - {e}")))?;
        Ok(req_rx.await?)
    }

    /// Send response for given request id
    pub(crate) async fn send_response(&self, resp: Response) -> Result<()> {
        self.tx
            .send(Cmd::SendResponse(resp))
            .await
            .map_err(|e| Error::NoMessageReceiver(format!("send_response failed - {e}")))?;
        Ok(())
    }
}

impl Term {
    /// Create a new terminal connection
    pub(crate) fn spawn(conn: Connection, pubsub: PubSubHandle) -> TermHandle {
        let (tx, rx) = mpsc::channel(32);
        let t = Term {
            tx: tx.downgrade(),
            rx,
            conn,
            req_queue: Default::default(),
            read_req_queue: Default::default(),
            pubsub,
            active_subs: Default::default(),
        };
        tokio::spawn(async move {
            if let Err(e) = t.run().await {
                error!("term error - {e}");
            }
        });
        TermHandle { tx }
    }

    /// Run the term task
    async fn run(mut self) -> Result<()> {
        loop {
            tokio::select! {
                msg = Term::read_msg(&mut self.conn) => {
                    self.handle_conn_msg(msg?).await?;
                },
                cmd = self.rx.recv() => {
                    let cmd = match cmd {
                        Some(cmd) => cmd,
                        None => break,
                    };
                    match cmd {
                        Cmd::SendResponse(resp) => {
                            self.conn.send_resp(resp).await
                                .map_err(|e| Error::IOError(format!("{}", e)))?;
                        }
                        Cmd::ReadRequest(req_tx) => {
                            match self.req_queue.pop_front() {
                                Some(req) => {
                                    let _ = req_tx.send(req);
                                }
                                None => {
                                    self.read_req_queue.push_back(req_tx);
                                }
                            }
                        },
                        Cmd::NotifySubscriptionUpdate(topic, contents) => {
                            self.handle_sub_update(topic, contents).await?;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Async task to poll for new messages
    async fn read_msg(conn: &mut Connection) -> Result<Message> {
        conn.recv()
            .await
            .ok_or(Error::ConnectionClosed)?
            .map_err(|e| Error::IOError(format!("{e}")))
    }

    /// Handle a message from connection
    async fn handle_conn_msg(&mut self, msg: Message) -> Result<()> {
        match msg {
            Message::Request(req) => {
                if let Some(tx) = self.read_req_queue.pop_front() {
                    let _ = tx.send(req);
                } else {
                    self.req_queue.push_back(req);
                }
            }
            Message::SubscriptionStart(req) => {
                self.setup_subscription(req).await?;
            }
            Message::SubscriptionEnd(topic) => {
                self.teardown_subscription(topic).await?;
            }
            _ => error!("Received unexpected msg over conn - {msg:?}"),
        }
        Ok(())
    }

    /// Setup a new subscription
    async fn setup_subscription(&mut self, req: SubscriptionRequest) -> Result<()> {
        info!("setup_subscription {req:?}");
        let mut sub = self.pubsub.subscribe(&req.topic).await?;

        // TODO: Idiom for streaming result from =Subscription= to another sink via async task for proc subs + term subs
        let tx = self.tx.clone();
        let topic = req.topic.clone();
        let sub_task = tokio::spawn(async move {
            while let Some(ev) = sub.recv().await {
                if let Some(tx) = tx.upgrade() {
                    let form: Form = match ev.try_into() {
                        Ok(f) => f,
                        Err(e) => {
                            error!("Unable to notify subscription update - {e}");
                            continue;
                        }
                    };
                    let _ = tx
                        .send(Cmd::NotifySubscriptionUpdate(topic.clone(), form))
                        .await;
                }
            }
        });

        self.active_subs.insert(req.topic, sub_task);

        Ok(())
    }

    /// Handle notification of subscription update
    async fn handle_sub_update(&mut self, topic: KeywordId, contents: Form) -> Result<()> {
        info!("handle_sub_update {topic} {contents}");
        self.conn
            .send(&Message::SubscriptionUpdate(SubscriptionUpdate {
                topic,
                contents,
            }))
            .await
            .map_err(|e| Error::IOError(format!("{}", e)))
    }

    /// Teardown a subscription for topic
    async fn teardown_subscription(&mut self, topic: KeywordId) -> Result<()> {
        info!("teardown_subscription {topic}");
        match self.active_subs.remove(&topic) {
            None => {
                error!("Unable to teardown - missing active sub for {topic}");
            }
            Some(task) => {
                task.abort();
            }
        }
        Ok(())
    }
}

impl std::cmp::PartialEq for TermHandle {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(&self.tx, &other.tx)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        rt::{program::Form, pubsub::PubSub},
        Client, Val,
    };

    use super::*;
    use assert_matches::assert_matches;
    use std::time::Duration;
    use tokio::{task::yield_now, time::timeout};

    #[tokio::test]
    async fn read_req_no_request() {
        let (rt, _client) = Connection::pair().unwrap();
        let t = Term::spawn(rt, PubSub::spawn());

        timeout(Duration::from_secs(0), t.read_request())
            .await
            .expect_err("Expect timeout");
    }

    // TODO: How to test interleaving of oneshot channel request / response after main terminal loop but before oneshot receiver recv?
    #[tokio::test]
    async fn read_request_sequencing() {
        // Send client.send_req before Cmd::ReadRequest
        {
            let (rt, mut client) = Connection::pair().unwrap();
            let t = Term::spawn(rt, PubSub::spawn());

            let (req_tx, req_rx) = oneshot::channel();

            client
                .send_req(Request {
                    id: 10,
                    contents: Form::string("Hello"),
                })
                .await
                .unwrap();

            yield_now().await;
            t.tx.send(Cmd::ReadRequest(req_tx)).await.unwrap();

            let req = req_rx.await.expect("Should receive req");

            assert_eq!(
                req,
                Request {
                    id: 10,
                    contents: Form::string("Hello")
                }
            );
        }

        // Send before Cmd::ReadRequest client.send_req
        {
            let (rt, mut client) = Connection::pair().unwrap();
            let t = Term::spawn(rt, PubSub::spawn());

            let (req_tx, req_rx) = oneshot::channel();

            t.tx.send(Cmd::ReadRequest(req_tx)).await.unwrap();
            client
                .send_req(Request {
                    id: 10,
                    contents: Form::string("Hello"),
                })
                .await
                .unwrap();

            let req = req_rx.await.expect("Should receive req");

            assert_eq!(
                req,
                Request {
                    id: 10,
                    contents: Form::string("Hello")
                }
            );
        }
    }

    #[tokio::test]
    async fn read_request_in_sequence() {
        let (rt, mut client) = Connection::pair().unwrap();

        let t = Term::spawn(rt, PubSub::spawn());

        tokio::spawn(async move {
            for i in 0..5 {
                client
                    .send_req(Request {
                        id: i,
                        contents: Form::Int(i.try_into().unwrap()),
                    })
                    .await
                    .unwrap();
            }
            loop {
                yield_now().await; // don't drop client
            }
        });

        let term_task = tokio::spawn(async move {
            let mut reqs = vec![];
            for _ in 0..5 {
                let req = t.read_request().await.unwrap();
                reqs.push(req);
            }
            reqs
        });

        let reqs = term_task.await.unwrap();
        let expected = (0..5)
            .map(|i| Request {
                id: i,
                contents: Form::Int(i.try_into().unwrap()),
            })
            .collect::<Vec<_>>();
        assert_eq!(reqs, expected, "Requests are returned in order");
    }

    #[tokio::test]
    async fn read_request_dropped_remote() {
        let (remote, local) = Connection::pair().unwrap();
        let t = Term::spawn(local, PubSub::spawn());

        let t_task = tokio::spawn(async move { t.read_request().await });

        drop(remote);

        let res = t_task.await.unwrap();
        assert_matches!(res, Err(_));
    }

    #[tokio::test]
    async fn send_response() {
        let (local, mut remote) = Connection::pair().unwrap();
        let t = Term::spawn(local, PubSub::spawn());

        t.send_response(Response {
            req_id: 99,
            contents: Ok(Form::string("Hello")),
        })
        .await
        .unwrap();

        let received = remote
            .recv_resp()
            .await
            .expect("Should be Some")
            .expect("Should be Ok");
        assert_eq!(
            received,
            Response {
                req_id: 99,
                contents: Ok(Form::string("Hello"))
            }
        );
    }

    #[tokio::test]
    async fn term_subscription() {
        let (runtime, client) = Connection::pair().unwrap();
        let client = Client::new(client);
        let pubsub = PubSub::spawn();
        let _t = Term::spawn(runtime, pubsub.clone());

        let client_task = tokio::spawn(async move {
            let mut client_sub = client.subscribe(KeywordId::from("my_topic")).await.unwrap();
            let mut seen = vec![];
            while seen.len() < 3 {
                seen.push(client_sub.recv().await.unwrap());
            }
            seen
        });

        // Publish increasing
        tokio::spawn(async move {
            let mut count = 0;
            loop {
                pubsub
                    .publish(&KeywordId::from("my_topic"), Val::Int(count))
                    .await
                    .unwrap();
                count += 1;
                yield_now().await;
            }
        });

        let res = client_task.await.unwrap();

        assert_matches!(res[..],
                        [Form::Int(a), Form::Int(b), Form::Int(c)] if b == a + 1 && c == b + 1,
                        "client subscription should observe a sequence of 3 increasing numbers");
    }
}
