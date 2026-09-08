mod codegen;
pub mod debug;
mod error;
mod lex;
pub mod macros;
mod parse;
mod pretty;
mod quasiquote;
mod run;
pub mod source;

pub mod builtin;
pub mod env;
pub mod fiber;
pub mod kwargs;
pub mod pmatch;
pub mod types;

pub use builtin::Ref;
pub use codegen::compile;
pub use codegen::Inst;
pub use env::Env;
pub use error::Error;
pub use fiber::Fiber;
pub use fiber::Signal;
pub use fiber::Status;
pub use parse::parse;
pub use parse::parse_script;
pub use parse::parse_source;
pub use pmatch::Pattern;
pub use run::run;
pub use types::Bytecode;
pub use types::Extern;
pub use types::Form;
pub use types::KeywordId;
pub use types::Lambda;
pub use types::Locals;
pub use types::NativeAsyncFn;
pub use types::NativeFn;
pub use types::NativeFnOp;
pub use types::SymbolId;
pub use types::Val;

pub type Result<T> = std::result::Result<T, Error>;

/// Default target width for human-readable values in Lyric and its clients.
pub const DEFAULT_PRINT_WIDTH: usize = 90;
