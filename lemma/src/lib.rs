mod env;
mod error;
mod eval;
mod form;
mod lex;
mod parse;
mod value;

pub mod lang;

pub use env::Env;
pub use error::Error;
pub use eval::eval;
pub use eval::eval_expr;
pub use form::Form;
pub use form::KeywordId;
pub use form::SymbolId;
pub use value::Lambda;
pub use value::SpecialForm;
pub use value::Value;

pub type Result<T> = std::result::Result<T, Error>;
