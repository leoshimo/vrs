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
}

/// Outgoing Requests
#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct Request {
    /// Unique ID assigned to request
    pub req_id: u32,
    /// Contents of request
    pub contents: lemma::Expr,
}

/// Incoming response
#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct Response {
    /// Unique ID of request this response is for
    pub req_id: u32,
    /// Contents of response
    pub contents: Result<lemma::Expr, Error>,
}

/// Error Type
#[derive(thiserror::Error, Debug, Deserialize, Serialize, PartialEq)]
pub enum Error {
    #[error("Error evaluating expression - {0}")]
    EvaluationError(#[from] lemma::Error),

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

    pub async fn send(&mut self, msg: &Message) -> Result<(), std::io::Error> {
        debug!("send msg={:?}", msg);
        let mut buf = BytesMut::new();
        let data = serde_json::to_string(msg).expect("message failed to serialize");
        buf.put(data.as_bytes());
        self.stream.send(buf.freeze()).await
    }

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

    pub async fn shutdown(self) -> Result<(), std::io::Error> {
        debug!("shutdown {:?}", self);
        self.stream.into_inner().shutdown().await
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
