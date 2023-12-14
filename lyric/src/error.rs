use crate::SymbolId;
use serde::{Deserialize, Serialize};

#[derive(thiserror::Error, Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum Error {
    #[error("Incomplete expression - {0}")]
    IncompleteExpression(String),

    #[error("Invalid expression - {0}")]
    InvalidExpression(String),

    #[error("Undefined symbol - {0}")]
    UndefinedSymbol(SymbolId),

    #[error("Unexpected arguments - {0}")]
    UnexpectedArguments(String),

    #[error("Unexpected type - {0}")]
    UnexpectedType(String),

    /// Unexpected state on stack
    #[error("Unexpected stack state - {0}")]
    UnexpectedStack(String),

    #[error("Unexpected resume of fiber - {0}")]
    UnexpectedResume(String),

    #[error("Invalid pattern match")]
    InvalidPatternMatch,
}
