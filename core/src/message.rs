//! Messages sent between client and runtime
use serde::{Deserialize, Serialize};

/// Outgoing Requests
#[derive(Debug, Deserialize, Serialize)]
pub struct Request {
    /// Unique ID assigned to request
    pub req_id: u32,
    /// Contents of request
    pub contents: lemma::Form,
}

/// Incoming response
#[derive(Debug, Deserialize, Serialize)]
pub struct Response {
    /// Unique ID of request this response is for
    pub req_id: u32,
    /// Contents of response
    pub contents: lemma::Form,
}
