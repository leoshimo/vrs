// v2 runtime
mod error;
mod kernel;
mod process;
mod subscription;

#[allow(clippy::module_inception)]
mod runtime;

pub use error::Error;
pub type Result<T> = std::result::Result<T, Error>;
pub use process::{ProcessHandle, ProcessId};
pub use runtime::Runtime;
