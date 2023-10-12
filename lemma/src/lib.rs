mod codegen;
mod env;
mod error;
mod lex;
mod parse;

pub mod fiber;
pub mod types;
pub use codegen::compile;

pub use env::Env;
pub use error::Error;
pub use parse::parse;
pub use types::Form;
pub use types::KeywordId;
pub use types::Lambda;
pub use types::NativeFunc;
pub use types::SymbolId;
pub use types::Val;

pub type Result<T> = std::result::Result<T, Error>;

pub(crate) use codegen::Inst;
