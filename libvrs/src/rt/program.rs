//! Program that specifies a process
use super::proc::ProcessId;
use super::proc_io::IOCmd;

/// Values produced by processes
pub type Val = lyric::Val<Extern, Locals>;

/// Environment used by proc
pub type Env = lyric::Env<Extern, Locals>;

/// Fibers for processes
pub type Fiber = lyric::Fiber<Extern, Locals>;

/// Pattern matches for processes
pub type Pattern = lyric::Pattern<Extern, Locals>;

/// Lambda for processes
pub type Lambda = lyric::Lambda<Extern, Locals>;

/// NativeFn type for Process bindings
pub type NativeFn = lyric::NativeFn<Extern, Locals>;

/// NativeFnOp for Process
pub type NativeFnOp = lyric::NativeFnOp<Extern, Locals>;

/// Extern type between Fiber and hosting Process
#[derive(Debug, Clone, PartialEq)]
pub enum Extern {
    ProcessId(ProcessId),
    IOCmd(Box<IOCmd>),
}

/// Locals for Process Fiber
#[derive(Debug, Clone, PartialEq)]
pub struct Locals {
    /// Id of process owning fiber
    pub(crate) pid: ProcessId,
}

/// Program used to spawn new processes
#[derive(Debug)]
pub struct Program {}
