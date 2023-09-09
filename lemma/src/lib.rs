pub mod parse;

mod eval;
mod expr;
mod lex;

pub use expr::Expr;

// TODO: Replace lazy errors with more structured errs
#[derive(thiserror::Error, Debug, PartialEq, Clone)]
pub enum Error {
    #[error("Failed to lex - {0}")]
    FailedToLex(String),

    #[error("Failed to parse - {0}")]
    FailedToParse(String),

    #[error("Empty expression")]
    EmptyExpression,
}

pub type Result<T> = std::result::Result<T, Error>;
