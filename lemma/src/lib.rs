mod env;
mod error;
mod eval;
mod form;
mod lex;
mod parse;

pub mod lang;

pub use env::Env;
pub use error::Error;
pub use eval::eval;
pub use eval::eval_expr;
pub use form::Form;
pub use form::KeywordId;
pub use form::Lambda;
pub use form::SpecialForm;
pub use form::SymbolId;
pub use parse::parse;

pub type Result<T> = std::result::Result<T, Error>;
