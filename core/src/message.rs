//! Messages sent over the wire
use serde::{Deserialize, Serialize};

/// The message
#[derive(Deserialize, Serialize)]
pub struct Message(pub serde_json::Value);

impl Message {
    pub fn new(contents: serde_json::Value) -> Self {
        Message(contents)
    }
}
