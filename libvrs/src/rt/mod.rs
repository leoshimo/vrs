mod bindings;
mod error;
mod kernel;
pub mod program;
pub(crate) mod pubsub;
mod registry;
mod runtime;
mod term;

mod mailbox;
mod peer;
mod proc;
mod remote;

pub use error::Error;
pub type Result<T> = std::result::Result<T, Error>;
pub use proc::{Process, ProcessExit, ProcessHandle, ProcessId, ProcessResult, ProcessSet};
pub use runtime::{Runtime, DEFAULT_NODE_PORT};
