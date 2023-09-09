// TODO: Replace lazy errors with more structured errs
#[derive(thiserror::Error, Debug, PartialEq, Clone)]
pub enum Error {
    #[error("Failed to lex - {0}")]
    FailedToLex(String),

    #[error("Failed to parse - {0}")]
    FailedToParse(String),

    #[error("Empty expression")]
    EmptyExpression,

    #[error("Unexpected operator - {0}")]
    UnexpectedOperator(String),

    #[error("Undefined symbol - {0}")]
    UndefinedSymbol(String),
}
