mod legacy;

// v2 runtime
mod error;
mod kernel;

#[allow(clippy::module_inception)]
mod runtime;

pub mod v2 {
    pub use super::error::Error;
    pub use super::runtime::Runtime;
    pub type Result<T> = std::result::Result<T, Error>;
}

pub use legacy::*;
