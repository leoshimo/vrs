mod error;
mod kernel;
mod runtime;

mod mailbox;
mod proc;
mod proc_bindings;
mod proc_io;

pub use error::Error;
pub type Result<T> = std::result::Result<T, Error>;
pub use proc::{ProcessHandle, ProcessId};
pub use runtime::Runtime;
