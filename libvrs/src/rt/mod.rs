mod bindings;
mod error;
mod kernel;
pub mod program;
mod pubsub;
mod registry;
mod runtime;

mod mailbox;
mod proc;

pub use error::Error;
pub type Result<T> = std::result::Result<T, Error>;
pub use proc::{Process, ProcessExit, ProcessHandle, ProcessId, ProcessResult, ProcessSet};
pub use runtime::Runtime;
