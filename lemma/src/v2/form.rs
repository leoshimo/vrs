//! Represents data types in Lemma
use serde::{Deserialize, Serialize};

/// Values in Lemma
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Form {
    Int(i32),
    String(String),
}

impl Form {
    /// Create new Form::String for given string
    pub fn string(inner: &str) -> Self {
        Form::String(inner.to_string())
    }
}
