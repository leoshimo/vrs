mod legacy;

// v2 runtime
mod error;
mod kernel;
mod process;
mod subscription;

#[allow(clippy::module_inception)]
mod runtime;

pub mod v2 {
    pub use super::error::Error;
    pub type Result<T> = std::result::Result<T, Error>;
    pub(crate) use super::process::ProcessId;
    pub(crate) use super::process::WeakProcessHandle;
    pub use super::runtime::Runtime;
}

pub use legacy::*;
