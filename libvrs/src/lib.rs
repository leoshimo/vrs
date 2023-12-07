use std::path::PathBuf;

mod connection;
pub use connection::{Connection, Request, Response};

mod client;
pub use client::Client;

mod rt;
pub use rt::program::{
    Bytecode, Env, Extern, Lambda, Locals, NativeFn, NativeFnOp, Pattern, Program, Val,
};
pub use rt::{Error, ProcessExit, ProcessHandle, ProcessResult, Result, Runtime};

/// The path to runtime socket
pub fn runtime_socket() -> Option<PathBuf> {
    let home = dirs::runtime_dir().or_else(dirs::home_dir)?;
    Some(home.as_path().join("vrsd.socket"))
}
