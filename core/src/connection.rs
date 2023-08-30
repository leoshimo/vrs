//! Sending and receiving message over connection between client and runtime.

use bytes::{BufMut, BytesMut};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::net::UnixStream;
use tokio_util::codec::Framed;
use tokio_util::codec::LengthDelimitedCodec;

/// The message defining over-the-wire data between client and runtime
#[derive(Deserialize, Serialize)]
pub enum Message {
    /// Request message
    Request {
        request_id: u32,
        contents: serde_json::Value,
    },
    /// Response message for Request
    Response {
        request_id: u32,
        contents: serde_json::Value,
    },
}

/// Connection that can be used to send [crate::connection::Message]
pub struct Connection {
    stream: Framed<UnixStream, LengthDelimitedCodec>,
}

impl Connection {
    pub fn new(stream: UnixStream) -> Self {
        let stream = Framed::new(stream, LengthDelimitedCodec::new());
        Connection { stream }
    }

    pub async fn send(&mut self, msg: &Message) -> Result<(), std::io::Error> {
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

        Some(Ok(msg))
    }
}
