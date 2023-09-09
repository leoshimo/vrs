mod env;
mod error;
mod eval;
mod form;
mod lex;
mod parse;
mod value;

pub use crate::env::Env;
pub use crate::error::Error;
pub use crate::eval::eval_expr;
pub use crate::form::Form;
pub use crate::form::SymbolId;
pub use crate::value::Value;

pub type Result<T> = std::result::Result<T, Error>;
