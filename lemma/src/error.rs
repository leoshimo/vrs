use crate::SymbolId;
use serde::{Deserialize, Serialize};

#[derive(thiserror::Error, Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum Error {
    #[error("Failed to lex - {0}")]
    FailedToLex(String),

    #[error("Failed to parse - {0}")]
    FailedToParse(String),

    #[error("Incomplete expression - {0}")]
    IncompleteExpression(String),

    #[error("Missing procedure")]
    MissingProcedure,

    #[error("Undefined symbol - {0}")]
    UndefinedSymbol(SymbolId),

    #[error("Unexpected arguments - {0}")]
    UnexpectedArguments(String),

    #[error("Invalid form to expr - {0}")]
    InvalidFormToExpr(String),
}
