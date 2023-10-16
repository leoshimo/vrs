mod codegen;
mod env;
mod error;
mod lex;
mod parse;

pub mod fiber;
pub mod types;

pub use codegen::compile;
pub use codegen::Inst;
pub use env::Env;
pub use error::Error;
pub use fiber::Fiber;
pub use fiber::FiberState;
pub use parse::parse;
pub use types::Form;
pub use types::KeywordId;
pub use types::Lambda;
pub use types::NativeFn;
pub use types::NativeFnVal;
pub use types::SymbolId;
pub use types::Val;

pub type Result<T> = std::result::Result<T, Error>;
