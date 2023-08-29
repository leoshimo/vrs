//! Connection wraps over bidirectional stream between client and runtime
use crate::message::Message;
use bytes::{BufMut, BytesMut};
use futures::{SinkExt, StreamExt};
use tokio::net::UnixStream;
use tokio_util::codec::Framed;
use tokio_util::codec::LengthDelimitedCodec;

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

        let json = serde_json::from_slice::<serde_json::Value>(&bytes);
        let json = match json {
            Ok(json) => json,
            Err(e) => {
                return Some(Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Unable to decode to json - {}", e),
                )))
            }
        };

        Some(Ok(Message::new(json)))
    }
}
