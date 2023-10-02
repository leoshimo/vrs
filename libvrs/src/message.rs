//! Messages sent between client and runtime
use serde::{Deserialize, Serialize};

/// Outgoing Requests
#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct Request {
    /// Unique ID assigned to request
    pub req_id: u32,
    /// Contents of request
    pub contents: lemma::Form,
}

/// Incoming response
#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct Response {
    /// Unique ID of request this response is for
    pub req_id: u32,
    /// Contents of response
    pub contents: Result<lemma::Form, Error>,
}

/// Error Type
#[derive(thiserror::Error, Debug, Deserialize, Serialize, PartialEq)]
pub enum Error {
    #[error("Error evaluating expression - {0}")]
    EvaluationError(#[from] lemma::Error),

    #[error("Unexpected error")]
    UnexpectedError,
}
