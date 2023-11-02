//! Program that specifies a process

use lyric::{Error, Result, SymbolId};

use super::binding;
use super::proc::ProcessId;
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

    {
        e.bind_native(SymbolId::from("recv_req"), binding::recv_req_fn())
            .bind_native(SymbolId::from("send_resp"), binding::send_resp_fn());
    }

    {
        e.bind_native(SymbolId::from("recv"), binding::recv_fn())
            .bind_native(SymbolId::from("ls-msgs"), binding::ls_msgs_fn())
            .bind_native(SymbolId::from("send"), binding::send_fn())
            .bind_lambda(SymbolId::from("call"), binding::call_fn());
    }

    {
        e.bind_native(SymbolId::from("kill"), binding::kill_fn())
            .bind_native(SymbolId::from("pid"), binding::pid_fn())
            .bind_native(SymbolId::from("ps"), binding::ps_fn())
            .bind_native(SymbolId::from("self"), binding::self_fn())
            .bind_native(SymbolId::from("sleep"), binding::sleep_fn())
            .bind_native(SymbolId::from("spawn"), binding::spawn_fn());
    }

    {
        e.bind_native(SymbolId::from("exec"), binding::exec_fn())
            .bind_native(SymbolId::from("shell_expand"), binding::shell_expand_fn());
    }

    {
        e.bind_lambda(SymbolId::from("open_app"), binding::open_app_fn())
            .bind_lambda(SymbolId::from("open_file"), binding::open_file_fn())
            .bind_lambda(SymbolId::from("open_url"), binding::open_url_fn());
    }

    {
        e.bind_native(SymbolId::from("register"), binding::register_fn())
            .bind_native(SymbolId::from("find-srv"), binding::find_srv_fn())
            .bind_native(SymbolId::from("ls-srv"), binding::ls_srv_fn());
    }

    e
}
