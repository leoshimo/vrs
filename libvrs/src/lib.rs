use std::path::PathBuf;

pub mod client;
pub mod connection;
pub mod rt;

pub use client::Client;
pub use connection::{Connection, Request, Response};
pub use rt::ProcessExit;
pub use rt::ProcessHandle;
pub use rt::ProcessResult;
pub use rt::Program;
pub use rt::Runtime;

/// The path to runtime socket
pub fn runtime_socket() -> Option<PathBuf> {
    let home = dirs::runtime_dir().or_else(dirs::home_dir)?;
    Some(home.as_path().join("vrsd.socket"))
}
