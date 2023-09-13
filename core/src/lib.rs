use std::path::PathBuf;

mod machine;
mod task;

// TODO Use reexports instead
pub mod client;
pub mod connection;
pub mod message;
pub mod runtime;

pub use client::Client;
pub use connection::Connection;
pub use message::{Request, Response};
pub use runtime::Runtime;

/// The path to runtime socket
pub fn runtime_socket() -> Option<PathBuf> {
    let home = dirs::runtime_dir().or_else(dirs::home_dir)?;
    Some(home.as_path().join("vrsd.socket"))
}
