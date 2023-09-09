mod env;
mod error;
mod eval;
mod expr;
mod lex;
mod parse;

pub use crate::env::Env;
pub use crate::error::Error;
pub use crate::expr::Expr;

pub type Result<T> = std::result::Result<T, Error>;
