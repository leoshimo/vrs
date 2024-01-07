use std::path::PathBuf;

mod client;
mod connection;
mod rt;

pub use connection::{Connection, Request, Response};

pub use client::Client;

pub use rt::program::{
    Bytecode, Env, Extern, Form, KeywordId, Lambda, Locals, NativeFn, NativeFnOp, Pattern, Program,
    Val,
};
pub use rt::{Error, ProcessExit, ProcessHandle, ProcessResult, Result, Runtime}; // TODO: Should rt reexport from lib?

/// The path to runtime socket
pub fn runtime_socket() -> PathBuf {
    let home = dirs::runtime_dir()
        .or_else(dirs::home_dir)
        .expect("Could not retrieve find runtime or home directory");
    home.as_path().join(runtime_socket_name())
}

/// The name of runtime socket
pub fn runtime_socket_name() -> &'static str {
    if cfg!(debug_assertions) {
        "vrsd-debug.socket"
    } else {
        "vrsd.socket"
    }
}
