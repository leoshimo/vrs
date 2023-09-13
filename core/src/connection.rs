//! Sending and receiving message over connection between client and runtime.

use std::os::fd::AsRawFd;

use crate::message::{Request, Response};
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
#[derive(Debug, Deserialize, Serialize)]
pub enum Message {
    Request(Request),
    Response(Response),
}

impl Connection {
    pub fn new(stream: UnixStream) -> Self {
        let stream = Framed::new(stream, LengthDelimitedCodec::new());
        Connection { stream }
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
    use super::Connection;

    /// Returns a connection fixture for tests
    pub fn conn_fixture() -> (Connection, Connection) {
        let (local, remote) = tokio::net::UnixStream::pair().unwrap();
        (Connection::new(local), Connection::new(remote))
    }
}
