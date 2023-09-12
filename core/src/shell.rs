//! Client shell hosting vrsjmp event loop
use std::collections::HashMap;

use crate::connection::{Connection, Message};
use crate::message::{Request, Response};
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, error};

/// Handle for client shell
#[derive(Clone, Debug)]
pub struct Shell {
    /// Sender half to send messages to event loop
    tx: mpsc::Sender<Event>,
}

impl Shell {
    /// Create new shell
    pub fn new(conn: Connection) -> Self {
        let (tx, rx) = mpsc::channel(32);
        let evloop = EventLoop::new(conn, rx);
        tokio::spawn(run(evloop));
        Self { tx }
    }

    /// Dispatch a request
    pub async fn request(&mut self, contents: lemma::Form) -> Result<Response, Error> {
        debug!("send request contents = {:?}", contents);
        let (resp_tx, resp_rx) = oneshot::channel();
        let ev = Event::SendRequest { contents, resp_tx };
        self.tx.send(ev).await?;
        Ok(resp_rx.await?)
    }
}

/// Errors from interacting with [Shell]
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Failed to send request")]
    RequestSendError(#[from] tokio::sync::mpsc::error::SendError<Event>),

    #[error("Failed to receive response to request")]
    RequestRecvError(#[from] tokio::sync::oneshot::error::RecvError),

    #[error("Receied unexpected message - {0}")]
    UnexpectedMessage(String),
}

/// Events processed in event loop
#[derive(Debug)]
pub enum Event {
    /// Event for sending request to remote
    SendRequest {
        contents: lemma::Form,
        resp_tx: oneshot::Sender<Response>,
    },
    /// Event when receiving response from remote
    RecvResponse(Response),
    /// Event when reading on connection results in IO error
    RecvError(std::io::Error),
    /// Event when receiving request from remote
    RecvRequest(Request),
}

/// Client side event loop
#[derive(Debug)]
struct EventLoop {
    /// The connection between runtime
    conn: Connection,
    /// Receiver for events from shell
    rx: mpsc::Receiver<Event>,
    /// Maps req_ids to Sender channel for responses
    inflight_reqs: HashMap<u32, oneshot::Sender<Response>>,
    /// Next request id to use
    next_req_id: u32,
}

impl EventLoop {
    fn new(conn: Connection, rx: mpsc::Receiver<Event>) -> Self {
        Self {
            conn,
            rx,
            next_req_id: 0,
            inflight_reqs: HashMap::new(),
        }
    }

    async fn handle_event(&mut self, e: Event) {
        debug!("received {:?}", e);
        use Event::*;
        match e {
            SendRequest { contents, resp_tx } => {
                let req = Request {
                    req_id: self.next_req_id,
                    contents,
                };
                self.next_req_id += 1;
                self.inflight_reqs.insert(req.req_id, resp_tx);
                let _ = self.conn.send(&Message::Request(req)).await;
            }
            RecvResponse(resp) => match self.inflight_reqs.remove(&resp.req_id) {
                Some(tx) => {
                    let _ = tx.send(resp);
                }
                None => {
                    error!("Received unexpected response for request - {:?}", resp);
                }
            },
            RecvError(e) => {
                error!("Encountered error - {}", e);
            }
            RecvRequest { .. } => panic!("Unimplemented - received request from runtime"),
        }
    }
}

impl From<Message> for Event {
    fn from(msg: Message) -> Self {
        match msg {
            Message::Response(resp) => Self::RecvResponse(resp),
            Message::Request(req) => Self::RecvRequest(req),
        }
    }
}

/// Start client side event loop
async fn run(mut evloop: EventLoop) {
    loop {
        let ev = tokio::select! {
            Some(e) = evloop.rx.recv() => e,
            Some(msg) = evloop.conn.recv() => {
                msg.map(Event::from).unwrap_or_else(Event::RecvError)
            }
        };
        evloop.handle_event(ev).await;
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::shell::Message;
    use tokio::net::UnixStream;

    #[tokio::test(flavor = "multi_thread")]
    async fn handle_request_response() {
        let (local, remote) = UnixStream::pair().unwrap();
        let local = Connection::new(local);
        let mut remote = Connection::new(remote);

        // Mock fake runtime that echos back requests
        tokio::spawn(async move {
            while let Some(msg) = remote.recv().await {
                if let Ok(Message::Request(req)) = msg {
                    let message = match req.contents {
                        lemma::Form::String(s) => s,
                        _ => todo!(),
                    };
                    let resp = Response {
                        req_id: req.req_id,
                        contents: lemma::Form::String(format!("reply {}", message)),
                    };
                    remote
                        .send(&Message::Response(resp))
                        .await
                        .expect("messsage should send");
                }
            }
        });

        let mut shell = Shell::new(local);
        let req = shell
            .request(lemma::Form::string("one"))
            .await
            .expect("Should receive reply");
        assert_eq!(req.contents, lemma::Form::string("reply one"));

        let req = shell
            .request(lemma::Form::string("two"))
            .await
            .expect("Should receive reply");
        assert_eq!(req.contents, lemma::Form::string("reply two"));

        let req = shell
            .request(lemma::Form::string("three"))
            .await
            .expect("Should receive reply");
        assert_eq!(req.contents, lemma::Form::string("reply three"));
    }
}
