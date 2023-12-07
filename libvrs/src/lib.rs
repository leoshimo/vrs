use std::path::PathBuf;

mod client;
mod connection;
mod rt;
mod subscription;

pub use connection::{Connection, Request, Response};

pub use client::Client;
pub use subscription::Subscription;

pub use rt::program::{
    Bytecode, Env, Extern, Lambda, Locals, NativeFn, NativeFnOp, Pattern, Program, Val,
};
pub use rt::{Error, ProcessExit, ProcessHandle, ProcessResult, Result, Runtime};

/// The path to runtime socket
pub fn runtime_socket() -> Option<PathBuf> {
    let home = dirs::runtime_dir().or_else(dirs::home_dir)?;
    Some(home.as_path().join("vrsd.socket"))
}
