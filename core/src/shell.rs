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
        let evloop = EventLoop {
            conn,
            rx,
            next_req_id: 0,
            inflight_reqs: HashMap::new(),
        };
        tokio::spawn(run(evloop));
        Self { tx }
    }

    /// Dispatch a request
    pub async fn request(&mut self, contents: serde_json::Value) -> Result<Response, Error> {
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

    #[error("Failed to receive response to reques")]
    RequestRecvError(#[from] tokio::sync::oneshot::error::RecvError),
}

/// Events processed in event loop
#[derive(Debug)]
pub enum Event {
    /// Event for sending request to remote
    SendRequest {
        contents: serde_json::Value,
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
    #[tracing::instrument(skip(self))]
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
