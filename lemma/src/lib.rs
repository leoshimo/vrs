mod env;
mod error;
mod form;
mod lex;
mod parse;
mod codegen;

pub mod fiber;
pub use codegen::compile;

pub use env::Env;
pub use error::Error;
pub use form::Expr;
pub use form::Form;
pub use form::KeywordId;
pub use form::Lambda;
pub use form::NativeFunc;
pub use form::SymbolId;
pub use parse::parse;

pub type Result<T> = std::result::Result<T, Error>;

pub(crate) use codegen::Inst;
