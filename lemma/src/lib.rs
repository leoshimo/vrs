pub mod lex;

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum Error {
    #[error("Failed to lex - {0}")]
    FailedToLex(String),
}

pub type Result<T> = std::result::Result<T, Error>;
