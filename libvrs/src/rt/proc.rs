use super::kernel::WeakKernelHandle;
use super::mailbox::Message;
use super::proc_io::ProcIO;
use super::program::{Extern, Fiber, Locals, Val};
use super::pubsub::PubSubHandle;
use super::registry::Registry;
use crate::rt::mailbox::{Mailbox, MailboxHandle};
use crate::rt::{Error, Result};
use crate::{Connection, Program};
use futures::future::{FutureExt, Shared};
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinSet;
use tracing::info;

/// Set of running processes
pub type ProcessSet = JoinSet<ProcessExit>;

/// IDs assigned to processes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProcessId(usize);

/// A running process in runtime
pub struct Process {
    id: ProcessId,
    prog: Program,
    locals: Locals,
    io: ProcIO,
}

/// A handle to [Process]
#[derive(Debug, Clone)]
pub struct ProcessHandle {
    id: ProcessId,
    hdl_tx: mpsc::Sender<Event>,
    mailbox: MailboxHandle,
    exit_rx: Shared<oneshot::Receiver<ProcessExit>>,
}

/// The result of process
#[derive(Debug, Clone, PartialEq)]
pub enum ProcessResult {
    /// Completed with value
    Done(Val),
    /// Cancelled for closed event loop
    Cancelled,
    /// Completed for disconnected connection
    Disconnected,
}

/// A record of process exiting
#[derive(Debug, Clone)]
pub struct ProcessExit {
    pub id: ProcessId,
    pub status: Result<ProcessResult>,
}

impl Process {
    /// Create a new process for program
    pub(crate) fn from_prog(id: ProcessId, prog: Program) -> Self {
        Self {
            id,
            prog,
            locals: Locals { pid: id },
            io: ProcIO::new(id),
        }
    }

    /// Set connection on process
    pub(crate) fn conn(mut self, conn: Connection) -> Self {
        self.io.conn(conn);
        self
    }

    /// Set kernel handle for process
    pub(crate) fn kernel(mut self, k: WeakKernelHandle) -> Self {
        self.io.kernel(k);
        self
    }

    /// Set registry handle for process
    pub(crate) fn registry(mut self, r: Registry) -> Self {
        self.io.registry(r);
        self
    }

    /// Set pubsub handle for process
    pub(crate) fn pubsub(mut self, pubsub: PubSubHandle) -> Self {
        self.io.pubsub(pubsub);
        self
    }

    /// Spawn a process
    pub(crate) fn spawn(mut self, procs: &mut ProcessSet) -> Result<ProcessHandle> {
        info!("proc spawn - {}", self.id);

        let (exit_tx, exit_rx) = oneshot::channel();
        let (msg_tx, mut msg_rx) = mpsc::channel(32);

        let mut fiber = self.prog.into_fiber(self.locals);

        let mailbox: MailboxHandle = Mailbox::spawn(self.id);
        self.io.mailbox(mailbox.clone());

        let proc_hdl = ProcessHandle {
            id: self.id,
            hdl_tx: msg_tx,
            exit_rx: exit_rx.shared(),
            mailbox,
        };

        self.io.handle(proc_hdl.clone());

        procs.spawn(async move {
            // TODO: Implement ProcessResult::Disconnected when Error::ConnectionClosed is returned
            // TODO: Use cancel token instead of msg_rx
            let exit = tokio::select! {
                res = lyric::run(&mut fiber) => {
                    match res {
                        Ok(v) => ProcessExit {
                            id: self.id,
                            status: Ok(ProcessResult::Done(v)),
                        },
                        Err(e) => ProcessExit {
                            id: self.id,
                            status: Err(Error::EvaluationError(e)),
                        }
                    }
                },
                Some(msg) = msg_rx.recv() => match msg {
                    Event::Kill => ProcessExit {
                        id: self.id,
                        status: Ok(ProcessResult::Cancelled)
                    }
                },
            };

            let _ = exit_tx.send(exit.clone());
            exit
        });

        Ok(proc_hdl)
    }

    #[allow(dead_code)]
    /// Handle a yield signal from fiber
    async fn handle_yield(fiber: &mut Fiber, val: Val, io: &mut ProcIO) -> Result<Val> {
        let iocmd = match val {
            Val::Extern(Extern::IOCmd(cmd)) => cmd,
            _ => return Err(Error::UnexpectedYield),
        };
        io.dispatch_io(fiber, *iocmd).await
    }
}

impl ProcessHandle {
    /// Get the ID
    pub fn id(&self) -> ProcessId {
        self.id
    }

    /// Send kill message to process. Effect is not immediate.
    pub(crate) async fn kill(&self) {
        let _ = self.hdl_tx.send(Event::Kill).await;
    }

    /// Wait for process to end
    pub async fn join(self) -> Result<ProcessExit> {
        Ok(self.exit_rx.await?)
    }

    /// Send a new message to process's mailbox
    pub(crate) async fn notify_message(&self, msg: Message) {
        let _ = self.mailbox.push(msg).await;
    }
}

#[derive(Debug)]
enum Event {
    Kill,
}

impl ProcessId {
    pub fn inner(&self) -> &usize {
        &self.0
    }
}

impl From<usize> for ProcessId {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl std::fmt::Display for ProcessId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<pid {}>", self.0)
    }
}

impl std::fmt::Display for Extern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Extern::ProcessId(pid) => write!(f, "{}", pid),
            Extern::IOCmd(_) => write!(f, "<iocmd>"),
        }
    }
}

impl std::fmt::Display for ProcessExit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.status {
            Ok(ProcessResult::Done(v)) => write!(f, "DONE - {v}"),
            Ok(ProcessResult::Disconnected) => write!(f, "DISCONNECTED"),
            Ok(ProcessResult::Cancelled) => write!(f, "CANCELLED"),
            Err(e) => write!(f, "ERROR - {e}"),
        }
    }
}

impl ProcessResult {
    pub fn unwrap(self) -> Val {
        match self {
            ProcessResult::Done(v) => v,
            _ => panic!("Unwrapping a process result that is not done"),
        }
    }
}

#[cfg(test)]
mod tests {

    use assert_matches::assert_matches;

    use super::*;

    #[tokio::test]
    async fn spawn_simple() {
        let mut procs = ProcessSet::new();
        let prog = Program::from_expr("\"Hello\"").unwrap();
        let _ = Process::from_prog(99.into(), prog)
            .spawn(&mut procs)
            .unwrap();

        let res = procs.join_next().await.unwrap().unwrap();
        assert_eq!(res.id, 99.into());
        assert_eq!(
            res.status.unwrap(),
            ProcessResult::Done(Val::string("Hello")),
        );
    }

    #[tokio::test]
    async fn processes_are_isolated() {
        let mut procs = ProcessSet::new();
        let prog = Program::from_expr("(def x 0)").unwrap();
        let _ = Process::from_prog(0.into(), prog)
            .spawn(&mut procs)
            .unwrap();
        let res = procs.join_next().await.unwrap().unwrap();
        assert_eq!(res.status.unwrap(), ProcessResult::Done(Val::Int(0)),);

        let prog = Program::from_expr("x").unwrap();
        let _ = Process::from_prog(0.into(), prog)
            .spawn(&mut procs)
            .unwrap();
        let res = procs.join_next().await.unwrap().unwrap();
        assert_matches!(
            res.status,
            Err(Error::EvaluationError(lyric::Error::UndefinedSymbol(_))),
            "processes should not share environment by default",
        );
    }

    #[tokio::test]
    async fn process_pid() {
        let mut procs = ProcessSet::new();

        let prog = Program::from_expr("(self)").unwrap();
        let hdl = Process::from_prog(99.into(), prog)
            .spawn(&mut procs)
            .unwrap();

        assert_eq!(hdl.id(), 99.into(), "ProcessHandle should have matching ID");

        let res = procs.join_next().await.unwrap().unwrap();
        assert_eq!(
            res.status.unwrap(),
            ProcessResult::Done(Val::Extern(Extern::ProcessId(99.into()))),
            "(self) should return assigned PID"
        );
    }

    // TODO: Implement + test preemption
    // #[tokio::test]
    // async fn drop_handle_ends_process() {
    //     let mut procs = ProcessSet::new();
    //     let handle = spawn(parse("(loop 0)").unwrap().into(), &mut procs).unwrap();
    //     drop(handle)
    //     assert_eq!(procs.join_next().await.unwrap().unwrap().unwrap(),
    //                ProcessResult::Cancelled);
    // }

    // TODO: Test that top-level yield of jibberish like (yield 1) results in process terminating w/ error
    // TODO: Test spawning invalid expressions - quote w/o any expression
    // TODO: Test that dropping process handle ends process
    // TODO: Test ProcessHandle::kill
    // TODO: Test `spawn` builtin (w/ correct capture / eval semantics)
}
