use crate::SymbolId;

#[derive(thiserror::Error, Debug, PartialEq, Clone)]
pub enum Error {
    #[error("Failed to lex - {0}")]
    FailedToLex(String),

    #[error("Failed to parse - {0}")]
    FailedToParse(String),

    #[error("Incomplete expression - {0}")]
    IncompleteExpression(String),

    #[error("Missing procedure")]
    MissingProcedure,

    #[error("Not a procedure - {0}")]
    NotAProcedure(crate::Form),

    #[error("Undefined symbol - {0}")]
    UndefinedSymbol(SymbolId),

    #[error("Unexpected arguments - {0}")]
    UnexpectedArguments(String),
}
