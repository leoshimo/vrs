#![allow(dead_code)]
//! Program that specifies a process

use lyric::{Error, Result, SymbolId};

use crate::ProcessHandle;

use super::bindings;
use super::kernel::WeakKernelHandle;
use super::proc::ProcessId;
use super::proc_io::IOCmd;
use super::pubsub::PubSubHandle;
use super::registry::Registry;

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
#[derive(Debug, Clone, PartialEq)]
pub struct Locals {
    /// Id of process owning fiber
    pub(crate) pid: ProcessId,
    /// Handle to kernel process
    pub(crate) kernel: Option<WeakKernelHandle>,
    /// Handle to process registry
    pub(crate) registry: Option<Registry>,
    /// Handle to pubsub
    pub(crate) pubsub: Option<PubSubHandle>,
    /// Handle to current process
    pub(crate) self_handle: Option<ProcessHandle>,
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

impl Locals {
    pub(crate) fn new(pid: ProcessId) -> Self {
        Self {
            pid,
            kernel: None,
            registry: None,
            pubsub: None,
            self_handle: None,
        }
    }

    pub(crate) fn kernel(&mut self, kernel: WeakKernelHandle) -> &mut Self {
        self.kernel = Some(kernel);
        self
    }

    pub(crate) fn registry(&mut self, registry: Registry) -> &mut Self {
        self.registry = Some(registry);
        self
    }

    pub(crate) fn pubsub(&mut self, pubsub: PubSubHandle) -> &mut Self {
        self.pubsub = Some(pubsub);
        self
    }

    pub(crate) fn handle(&mut self, handle: ProcessHandle) -> &mut Self {
        //
        self.self_handle = Some(handle);
        self
    }
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
        e.bind_native_async(SymbolId::from("recv"), bindings::recv_fn())
            .bind_native_async(SymbolId::from("ls-msgs"), bindings::ls_msgs_fn())
            .bind_native_async(SymbolId::from("send"), bindings::send_fn())
            .bind_lambda(SymbolId::from("call"), bindings::call_fn());
    }

    {
        e.bind_native(SymbolId::from("srv"), bindings::srv_fn())
            .bind_lambda(SymbolId::from("bind-srv"), bindings::bind_srv_fn())
            .bind_native(
                SymbolId::from("def-bind-interface"),
                bindings::def_bind_interface(),
            )
            .bind_native_async(SymbolId::from("info-srv"), bindings::info_srv_fn());
    }

    {
        e.bind_native_async(SymbolId::from("kill"), bindings::kill_fn())
            .bind_native(SymbolId::from("pid"), bindings::pid_fn())
            .bind_native_async(SymbolId::from("ps"), bindings::ps_fn())
            .bind_native(SymbolId::from("self"), bindings::self_fn())
            .bind_native_async(SymbolId::from("sleep"), bindings::sleep_fn())
            .bind_native_async(SymbolId::from("spawn"), bindings::spawn_fn());
    }

    {
        e.bind_native_async(SymbolId::from("exec"), bindings::exec_fn())
            .bind_native(SymbolId::from("shell_expand"), bindings::shell_expand_fn());
    }

    {
        e.bind_lambda(SymbolId::from("open_app"), bindings::open_app_fn())
            .bind_lambda(SymbolId::from("open_file"), bindings::open_file_fn())
            .bind_lambda(SymbolId::from("open_url"), bindings::open_url_fn());
    }

    {
        e.bind_native_async(SymbolId::from("register"), bindings::register_fn())
            .bind_lambda(SymbolId::from("find-srv"), bindings::find_srv_fn())
            .bind_native_async(SymbolId::from("ls-srv"), bindings::ls_srv_fn());
    }

    {
        e.bind_native_async(SymbolId::from("subscribe"), bindings::subscribe_fn())
            .bind_native_async(SymbolId::from("publish"), bindings::publish_fn());
    }

    e
}
