//! Sending and receiving message over connection between client and runtime.

use std::os::fd::AsRawFd;

use bytes::{BufMut, BytesMut};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;
use tokio::net::UnixStream;
use tokio_util::codec::Framed;
use tokio_util::codec::LengthDelimitedCodec;
use tracing::debug;

use crate::rt::program::Form;
use crate::rt::program::KeywordId;

/// Connection that can be used to send [crate::connection::Message]
/// TODO: Use AsyncRead + AsyncWrite traits instead of UnixStream
pub struct Connection {
    stream: Framed<UnixStream, LengthDelimitedCodec>,
}

/// Messages between client and runtime
#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub enum Message {
    Request(Request),
    Response(Response),
    SubscriptionRequest(SubscriptionRequest),
    SubscriptionUpdate(SubscriptionUpdate),
}

/// Outgoing Requests
#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct Request {
    /// Unique ID assigned to request
    pub id: u32,
    /// Contents of request
    pub contents: lyric::Form,
}

/// Client request for new subscriptions
#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct SubscriptionRequest {
    pub(crate) topic: KeywordId,
}

/// Runtime response for subscription topic changes
#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct SubscriptionUpdate {
    pub(crate) topic: KeywordId,
    pub(crate) contents: Form,
}

/// Incoming response
#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct Response {
    /// Unique ID of request this response is for
    pub req_id: u32,
    /// Contents of response
    pub contents: Result<lyric::Form, Error>,
}

/// Error Type
#[derive(thiserror::Error, Debug, Deserialize, Serialize, PartialEq)]
pub enum Error {
    #[error("Error evaluating expression - {0}")]
    EvaluationError(#[from] lyric::Error),

    #[error("Unexpected error")]
    UnexpectedError,
}

impl Connection {
    /// Returns a new [Connection] over given unix stream
    pub fn new(stream: UnixStream) -> Self {
        let stream = Framed::new(stream, LengthDelimitedCodec::new());
        Connection { stream }
    }

    /// Returns a pair of [Connection] connected to one another
    pub fn pair() -> tokio::io::Result<(Connection, Connection)> {
        let (local, remote) = tokio::net::UnixStream::pair()?;
        Ok((Connection::new(local), Connection::new(remote)))
    }

    /// Send a message
    pub async fn send(&mut self, msg: &Message) -> Result<(), std::io::Error> {
        debug!("send msg={:?}", msg);
        let mut buf = BytesMut::new();
        let data = serde_json::to_string(msg).expect("message failed to serialize");
        buf.put(data.as_bytes());
        self.stream.send(buf.freeze()).await
    }

    /// Receive a message
    pub async fn recv(&mut self) -> Option<Result<Message, std::io::Error>> {
        let bytes = self.stream.next().await?;
        let bytes = match bytes {
            Ok(bytes) => bytes,
            Err(e) => return Some(Err(e)),
        };

        let msg = serde_json::from_slice::<Message>(&bytes);
        let msg = match msg {
            Ok(msg) => msg,
            Err(e) => {
                return Some(Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Unable to decode to json - {}", e),
                )))
            }
        };
        debug!("recv msg={:?}", msg);
        Some(Ok(msg))
    }

    /// Shutdown the connection
    pub async fn shutdown(self) -> Result<(), std::io::Error> {
        debug!("shutdown {:?}", self);
        self.stream.into_inner().shutdown().await
    }
}

// Convenience APIs
impl Connection {
    /// Send a request
    pub async fn send_req(&mut self, req: Request) -> Result<(), std::io::Error> {
        self.send(&Message::Request(req)).await
    }

    /// Send a response
    pub async fn send_resp(&mut self, resp: Response) -> Result<(), std::io::Error> {
        self.send(&Message::Response(resp)).await
    }

    /// Receive a response
    pub async fn recv_req(&mut self) -> Option<Result<Request, std::io::Error>> {
        let msg = self.recv().await?;
        match msg {
            Ok(Message::Request(r)) => Some(Ok(r)),
            Ok(Message::Response(_))
            | Ok(Message::SubscriptionRequest(_))
            | Ok(Message::SubscriptionUpdate(_)) => {
                panic!("Expected request, but received response over connection")
            }
            Err(e) => Some(Err(e)),
        }
    }

    /// Receive a response
    pub async fn recv_resp(&mut self) -> Option<Result<Response, std::io::Error>> {
        let msg = self.recv().await?;
        match msg {
            Ok(Message::Response(r)) => Some(Ok(r)),
            Ok(Message::Request(_))
            | Ok(Message::SubscriptionRequest(_))
            | Ok(Message::SubscriptionUpdate(_)) => {
                panic!("Expected response, but received request over connection")
            }
            Err(e) => Some(Err(e)),
        }
    }
}

impl std::cmp::PartialEq for Connection {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.stream.get_ref(), other.stream.get_ref())
    }
}

impl std::fmt::Debug for Connection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let fd = self.stream.get_ref().as_raw_fd();
        write!(f, "Connection {{ fd: {} }}", fd)
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    /// Test that dropping one end of connection results in other end returning `None` on `recv` call
    #[tokio::test]
    async fn drop_remote() {
        let (mut local, remote) = Connection::pair().unwrap();

        drop(remote);

        assert!(
            local.recv().await.is_none(),
            "Dropped connection should return None"
        );
    }
}
