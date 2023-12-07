//! Headless client implementation for vrs runtime
use std::collections::HashMap;

use crate::connection::{Connection, Message, Request, Response};
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error};

/// Handle for vrs client
#[derive(Debug)]
pub struct Client {
    /// Sender half to send messages to shared task
    hdl_tx: mpsc::Sender<Event>,
    /// Cancellation token to shutdown async task
    cancel: CancellationToken,
}

/// Errors from interacting with [Client]
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Failed to send on mpsc - {0}")]
    FailedToSend(#[from] tokio::sync::mpsc::error::SendError<Event>),

    #[error("Failed to recv on oneshot - {0}")]
    FailedToRecv(#[from] tokio::sync::oneshot::error::RecvError),

    #[error("{0}")]
    IO(#[from] std::io::Error),

    #[error("Internal inconsistency - {0}")]
    Internal(String),

    #[error("Connection disconnected")]
    Disconnected,
}

/// Messages processed by event loop
#[derive(Debug)]
pub enum Event {
    /// Event for sending request to remote
    SendRequest {
        req: lyric::Form,
        resp_tx: oneshot::Sender<Response>,
    },
    /// Event when receiving response from remote
    RecvResponse(Response),
}

/// The state of active [Client]
#[derive(Debug)]
struct State {
    /// The connection to runtime
    conn: Connection,
    /// Maps req_ids to Sender channel for responses
    inflight_reqs: HashMap<u32, oneshot::Sender<Response>>,
    /// Next request id to use
    next_req_id: u32,
}

impl Client {
    /// Create new client from connection transport between client and runtime
    pub fn new(conn: Connection) -> Self {
        let (hdl_tx, hdl_rx) = mpsc::channel(32);
        let cancel = CancellationToken::new();

        let cancel_clone = cancel.clone();
        let state = State::new(conn);
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
    pub async fn request(&self, req: lyric::Form) -> Result<Response, Error> {
        debug!("request req = {}", req);
        let (resp_tx, resp_rx) = oneshot::channel();
        self.hdl_tx
            .send(Event::SendRequest { req, resp_tx })
            .await?;
        Ok(resp_rx.await?)
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
async fn run(mut state: State, mut hdl_rx: mpsc::Receiver<Event>) -> Result<(), Error> {
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
    fn new(conn: Connection) -> Self {
        Self {
            conn,
            next_req_id: 0,
            inflight_reqs: HashMap::new(),
        }
    }

    async fn handle_event(&mut self, e: Event) -> Result<(), Error> {
        debug!("handle_event e = {:?}", e);
        match e {
            Event::SendRequest {
                req: contents,
                resp_tx,
            } => self.handle_request(contents, resp_tx).await,
            Event::RecvResponse(resp) => self.handle_recv_response(resp).await,
        }
    }

    /// Handle a send request event
    async fn handle_request(
        &mut self,
        contents: lyric::Form,
        resp_tx: oneshot::Sender<Response>,
    ) -> Result<(), Error> {
        let req = Request {
            id: self.next_req_id,
            contents,
        };
        self.next_req_id += 1;
        self.inflight_reqs.insert(req.id, resp_tx);
        Ok(self.conn.send(&Message::Request(req)).await?)
    }

    /// Handle a recv response event
    async fn handle_recv_response(&mut self, resp: Response) -> Result<(), Error> {
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
}

impl TryFrom<Message> for Event {
    type Error = Error;
    fn try_from(value: Message) -> Result<Self, Self::Error> {
        match value {
            Message::Response(resp) => Ok(Self::RecvResponse(resp)),
            Message::Request(_) => Err(Error::Internal(
                "Client unexpectedly received Message::Request".to_string(),
            )),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::connection::Connection;
    use assert_matches::assert_matches;
    use lyric::Form;

    #[tokio::test]
    async fn request_response() {
        let (local, mut remote) = Connection::pair().unwrap();

        // Echo back response
        tokio::spawn(async move {
            while let Some(msg) = remote.recv().await {
                if let Ok(Message::Request(req)) = msg {
                    let resp = Response {
                        req_id: req.id,
                        contents: Ok(req.contents),
                    };
                    remote
                        .send(&Message::Response(resp))
                        .await
                        .expect("messsage should send");
                }
            }
        });

        let client = Client::new(local);
        let req1 = client.request(lyric::Form::string("one"));
        let req2 = client.request(lyric::Form::string("two"));
        let req3 = client.request(lyric::Form::string("three"));

        assert_matches!(
            tokio::try_join!(req2, req1, req3).unwrap(),
            (
                Response { contents: Ok(two), .. },
                Response { contents: Ok(one), .. },
                Response { contents: Ok(three), .. },
            ) if one == Form::string("one")
                && two == Form::string("two")
                && three == Form::string("three")
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
}
