mod codegen;
mod error;
mod lex;
mod parse;

pub mod builtin;
pub mod env;
pub mod fiber;
pub mod pmatch;
pub mod types;

pub use builtin::Ref;
pub use codegen::compile;
pub use codegen::Inst;
pub use env::Env;
pub use error::Error;
pub use fiber::Fiber;
pub use fiber::FiberState;
pub use parse::parse;
pub use pmatch::Pattern;
pub use types::Extern;
pub use types::Form;
pub use types::KeywordId;
pub use types::Lambda;
pub use types::Locals;
pub use types::NativeFn;
pub use types::NativeFnOp;
pub use types::SymbolId;
pub use types::Val;
pub use types::Bytecode;

pub type Result<T> = std::result::Result<T, Error>;
