#![allow(dead_code)]
//! Program that specifies a process

use lyric::{Error, Result, SymbolId};

use super::bindings;
use super::proc::ProcessId;
use super::proc_io::{IOCmd, ProcIO};

/// Program used to spawn new processes
#[derive(Debug, Clone)]
pub struct Program {
    code: Bytecode,
    env: Env,
}

/// Form for vrs
pub type Form = lyric::Form;

/// Reexport KeywordId for vrs
pub type KeywordId = lyric::KeywordId;

/// Values used in programs
pub type Val = lyric::Val<Extern, Locals>;

/// Environment used by programs
pub type Env = lyric::Env<Extern, Locals>;

/// Fibers for programs
pub type Fiber = lyric::Fiber<Extern, Locals>;

/// Pattern matches for programs
pub type Pattern = lyric::Pattern<Extern, Locals>;

/// Lambda for programs
pub type Lambda = lyric::Lambda<Extern, Locals>;

/// NativeFn type for program bindings
pub type NativeFn = lyric::NativeFn<Extern, Locals>;

/// NativeFnOp for program
pub type NativeFnOp = lyric::NativeFnOp<Extern, Locals>;

/// NativeAsyncFn type for program bindings
pub type NativeAsyncFn = lyric::NativeAsyncFn<Extern, Locals>;

/// Bytecode
pub type Bytecode = lyric::Bytecode<Extern, Locals>;

/// Extern type between Fiber and hosting program
#[derive(Debug, Clone, PartialEq)]
pub enum Extern {
    ProcessId(ProcessId),
    IOCmd(Box<IOCmd>),
}

/// Locals for Program Fiber
#[derive(Debug, Clone)]
pub struct Locals {
    /// Id of process owning fiber
    pub(crate) pid: ProcessId,
    /// IO sources
    pub(crate) io: ProcIO,
}

impl std::cmp::PartialEq for Locals {
    fn eq(&self, other: &Self) -> bool {
        use std::ptr;
        self.pid == other.pid && ptr::eq(&self.io, &other.io)
    }
}

impl Program {
    pub fn from_bytecode(code: Bytecode) -> Self {
        Self {
            code,
            env: program_env(),
        }
    }

    pub fn from_val(val: Val) -> Result<Self> {
        let code = lyric::compile(&val)?;
        Ok(Self::from_bytecode(code))
    }

    pub fn from_expr(expr: &str) -> Result<Self> {
        let val: Val = lyric::parse(expr)?.into();
        Self::from_val(val)
    }

    pub fn from_lambda(lambda: Lambda) -> Result<Self> {
        if !lambda.params.is_empty() {
            return Err(Error::UnexpectedArguments(
                "Program are created from zero arity lambdas".to_string(),
            ));
        }

        let mut prog = Self::from_bytecode(lambda.code);
        prog.env = match lambda.parent.as_ref() {
            Some(parent) => parent.as_ref().lock().unwrap().clone(),
            None => program_env(),
        };

        Ok(prog)
    }

    pub fn into_fiber(self, locals: Locals) -> Fiber {
        Fiber::from_bytecode(self.code, self.env, locals)
    }
}

/// Create a new program for connections
pub fn connection_program() -> Program {
    Program::from_expr("(loop (send_resp (try (eval (recv_req)))))")
        .expect("Connection program should compile")
}

impl PartialEq for Program {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}

/// Create new environment for programs
fn program_env() -> Env {
    let mut e = Env::standard();

    {
        e.bind_native(SymbolId::from("recv_req"), bindings::recv_req_fn())
            .bind_native(SymbolId::from("send_resp"), bindings::send_resp_fn());
    }

    {
        e.bind_native(SymbolId::from("recv"), bindings::recv_fn())
            .bind_native(SymbolId::from("ls-msgs"), bindings::ls_msgs_fn())
            .bind_native(SymbolId::from("send"), bindings::send_fn())
            .bind_lambda(SymbolId::from("call"), bindings::call_fn());
    }

    {
        e.bind_native(SymbolId::from("srv"), bindings::srv_fn())
            .bind_lambda(SymbolId::from("bind-srv"), bindings::bind_srv_fn())
            .bind_native(
                SymbolId::from("def-bind-interface"),
                bindings::def_bind_interface(),
            )
            .bind_native(SymbolId::from("info-srv"), bindings::info_srv_fn());
    }

    {
        e.bind_native(SymbolId::from("kill"), bindings::kill_fn())
            .bind_native(SymbolId::from("pid"), bindings::pid_fn())
            .bind_native(SymbolId::from("ps"), bindings::ps_fn())
            .bind_native(SymbolId::from("self"), bindings::self_fn())
            .bind_native_async(SymbolId::from("sleep"), bindings::sleep_fn())
            .bind_native(SymbolId::from("spawn"), bindings::spawn_fn());
    }

    {
        e.bind_native(SymbolId::from("exec"), bindings::exec_fn())
            .bind_native(SymbolId::from("shell_expand"), bindings::shell_expand_fn());
    }

    {
        e.bind_lambda(SymbolId::from("open_app"), bindings::open_app_fn())
            .bind_lambda(SymbolId::from("open_file"), bindings::open_file_fn())
            .bind_lambda(SymbolId::from("open_url"), bindings::open_url_fn());
    }

    {
        e.bind_native(SymbolId::from("register"), bindings::register_fn())
            .bind_native(SymbolId::from("find-srv"), bindings::find_srv_fn())
            .bind_native(SymbolId::from("ls-srv"), bindings::ls_srv_fn());
    }

    {
        e.bind_native(SymbolId::from("subscribe"), bindings::subscribe_fn())
            .bind_native(SymbolId::from("unsubscribe"), bindings::unsubscribe_fn())
            .bind_native(SymbolId::from("publish"), bindings::publish_fn());
    }

    e
}
