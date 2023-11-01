//! Program that specifies a process

use lyric::{Error, Result, SymbolId};

use super::proc::ProcessId;
use super::proc_bindings;
use super::proc_io::IOCmd;

/// Program used to spawn new processes
#[derive(Debug, Clone)]
pub struct Program {
    code: Bytecode,
    env: Env,
}

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

/// Bytecode
pub type Bytecode = lyric::Bytecode<Extern, Locals>;

/// Extern type between Fiber and hosting program
#[derive(Debug, Clone, PartialEq)]
pub enum Extern {
    ProcessId(ProcessId),
    IOCmd(Box<IOCmd>),
}

/// Locals for Program Fiber
#[derive(Debug, Clone, PartialEq)]
pub struct Locals {
    /// Id of process owning fiber
    pub(crate) pid: ProcessId,
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
        prog.env = Env::extend(&lambda.parent.unwrap());

        Ok(prog)
    }

    pub fn into_fiber(self, locals: Locals) -> Fiber {
        Fiber::from_bytecode(self.code, self.env, locals)
    }
}

/// Create a new program for connections
pub fn connection_program() -> Program {
    Program::from_expr("(loop (send_resp (peval (recv_req))))")
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

    e.bind_lambda(SymbolId::from("call"), proc_bindings::call_fn())
        .bind_lambda(SymbolId::from("open_app"), proc_bindings::open_app_fn())
        .bind_lambda(SymbolId::from("open_file"), proc_bindings::open_file_fn())
        .bind_lambda(SymbolId::from("open_url"), proc_bindings::open_url_fn())
        .bind_native(SymbolId::from("exec"), proc_bindings::exec_fn())
        .bind_native(SymbolId::from("kill"), proc_bindings::kill_fn())
        .bind_native(SymbolId::from("ls-msgs"), proc_bindings::ls_msgs_fn())
        .bind_native(SymbolId::from("pid"), proc_bindings::pid_fn())
        .bind_native(SymbolId::from("ps"), proc_bindings::ps_fn())
        .bind_native(SymbolId::from("recv"), proc_bindings::recv_fn())
        .bind_native(SymbolId::from("recv_req"), proc_bindings::recv_req_fn())
        .bind_native(SymbolId::from("self"), proc_bindings::self_fn())
        .bind_native(SymbolId::from("send"), proc_bindings::send_fn())
        .bind_native(SymbolId::from("send_resp"), proc_bindings::send_resp_fn())
        .bind_native(
            SymbolId::from("shell_expand"),
            proc_bindings::shell_expand_fn(),
        )
        .bind_native(SymbolId::from("sleep"), proc_bindings::sleep_fn())
        .bind_native(SymbolId::from("spawn"), proc_bindings::spawn_fn());

    e
}
