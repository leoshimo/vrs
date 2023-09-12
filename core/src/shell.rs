//! Client shell hosting vrsjmp event loop
use std::collections::HashMap;

use crate::connection::{Connection, Message};
use crate::message::{Request, Response};
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error};

/// Handle for client shell
#[derive(Clone, Debug)]
pub struct Shell {
    /// Sender half to send messages to event loop
    evloop_tx: mpsc::Sender<Event>,
    /// Cancellation token to shutdown event loop and shell handle
    evloop_cancel_token: CancellationToken,
}

impl Shell {
    /// Create new shell
    pub fn new(conn: Connection) -> Self {
        let (evloop_tx, evloop_rx) = mpsc::channel(32);
        let evloop_cancel_token = CancellationToken::new();
        let evloop = EventLoop::new(conn, evloop_cancel_token.clone());
        tokio::spawn(run(evloop, evloop_rx));
        Self {
            evloop_tx,
            evloop_cancel_token,
        }
    }

    /// Dispatch a request
    pub async fn request(&mut self, contents: lemma::Form) -> Result<Response, Error> {
        debug!("send request contents = {:?}", contents);
        let (resp_tx, resp_rx) = oneshot::channel();
        let ev = Event::SendRequest { contents, resp_tx };
        self.evloop_tx.send(ev).await?;
        Ok(resp_rx.await?)
    }

    /// Returns whether or not client shell is active
    pub fn is_active(&self) -> bool {
        !self.evloop_tx.is_closed() // tx is open as long as event loop is running
    }

    /// Shutdown
    pub async fn shutdown(&self) {
        self.evloop_cancel_token.cancel();
        let _ = self.evloop_tx.closed().await; // wait until shutdown
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
    /// Event when connection with runtime disconnects
    DisconnectedFromRuntime,
}

/// The state managed by client side event loop. The [Shell] is a handle to this [EventLoop]
#[derive(Debug)]
struct EventLoop {
    /// The connection between runtime
    conn: Connection,
    /// Maps req_ids to Sender channel for responses
    inflight_reqs: HashMap<u32, oneshot::Sender<Response>>,
    /// Next request id to use
    next_req_id: u32,
    /// Cancellation token used to shutdown event loop
    cancellation_token: CancellationToken,
}

impl EventLoop {
    fn new(conn: Connection, cancellation_token: CancellationToken) -> Self {
        Self {
            conn,
            next_req_id: 0,
            inflight_reqs: HashMap::new(),
            cancellation_token,
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
            DisconnectedFromRuntime => {
                debug!("shutting down event loop...");
                self.cancellation_token.cancel();
            }
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
async fn run(mut evloop: EventLoop, mut evloop_rx: mpsc::Receiver<Event>) {
    loop {
        let ev = tokio::select! {
            Some(e) = evloop_rx.recv() => e,
            msg = evloop.conn.recv() => match msg {
                Some(msg) => msg.map(Event::from).unwrap_or_else(Event::RecvError),
                None => Event::DisconnectedFromRuntime,
            },
            _ = evloop.cancellation_token.cancelled() => {
                break;
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

        // Remote echos back requests
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

    #[tokio::test(flavor = "multi_thread")]
    async fn handle_conn_drop() {
        use std::time::Duration;
        use tokio::time::timeout;

        let (local, remote) = UnixStream::pair().unwrap();
        let local = Connection::new(local);
        let mut remote = Connection::new(remote);

        // Remote drops `remote` after first request
        tokio::spawn(async move {
            let _ = remote.recv().await;
            // remote is dropped
        });

        let mut shell = Shell::new(local);

        let req = shell.request(lemma::Form::string("hi"));
        let resp = timeout(Duration::from_millis(10), req)
            .await
            .expect("Request should be notified that remote connection was dropped before timeout");

        assert!(matches!(resp, Err(Error::RequestRecvError(_))));
    }
}
