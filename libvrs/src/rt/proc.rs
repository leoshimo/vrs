use super::kernel::WeakKernelHandle;
use super::mailbox::Message;
use super::proc_io::ProcIO;
use super::program::{Extern, Locals, Val};
use super::registry::Registry;
use crate::rt::mailbox::{Mailbox, MailboxHandle};
use crate::rt::{Error, Result};
use crate::{Connection, Program};
use futures::future::{FutureExt, Shared};
use lyric::FiberState;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinSet;
use tracing::{debug, error, info};

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

        // TODO: Clean this up!
        self.io.handle(proc_hdl.clone());
        procs.spawn(async move {
            let exit: Result<_> = async {
                let mut io = self.io;
                let mut state = fiber.resume()?;
                loop {
                    match state {
                        FiberState::Done(v) => {
                            return Ok(ProcessExit {
                                id: self.id,
                                status: Ok(ProcessResult::Done(v)),
                            })
                        }
                        FiberState::Yield(v) => {
                            debug!("proc yield - {:?} {:?}", self.id, v);
                            tokio::select!(
                                Some(msg) = msg_rx.recv() => match msg {
                                    Event::Kill => return Ok(ProcessExit {
                                        id: self.id,
                                        status: Ok(ProcessResult::Cancelled)
                                    })
                                },
                                io_result = Self::handle_yield(v, &mut io) => {
                                    debug!("proc yield result - {:?} {:?}", self.id, io_result);

                                    let io_result = match io_result {
                                        Ok(r) => Ok(r),
                                        Err(Error::ConnectionClosed) => {
                                            return Ok(ProcessExit {
                                                id: self.id,
                                                status: Ok(ProcessResult::Disconnected)
                                            })
                                        }
                                        Err(e) => Err(e),
                                    }?;

                                    state = fiber.resume_from_yield(io_result)?;
                                }
                            );
                        }
                    }
                }
            }
            .await;

            let exit = match exit {
                Ok(exit) => {
                    info!("proc exit {} - {}", self.id, exit);
                    exit
                }
                Err(e) => {
                    error!("proc exit {} - {}", self.id, e);
                    ProcessExit {
                        id: self.id,
                        status: Err(e),
                    }
                }
            };

            let _ = exit_tx.send(exit.clone());
            exit
        });

        Ok(proc_hdl)
    }

    /// Handle a yield signal from fiber
    async fn handle_yield(val: Val, io: &mut ProcIO) -> Result<Val> {
        let iocmd = match val {
            Val::Extern(Extern::IOCmd(cmd)) => cmd,
            _ => return Err(Error::UnexpectedYield),
        };
        io.dispatch_io(*iocmd).await
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
    use lyric::Form;

    use crate::Request;

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
    async fn recv_req() {
        let (local, mut remote) = Connection::pair().unwrap();

        let mut procs = ProcessSet::new();
        let prog = Program::from_expr("(recv_req)").unwrap();
        let _ = Process::from_prog(0.into(), prog)
            .conn(local)
            .spawn(&mut procs);

        let _ = remote
            .send_req(Request {
                req_id: 0,
                contents: Form::string("Hello world"),
            })
            .await;

        let res = procs.join_next().await.unwrap().unwrap();
        assert_eq!(
            res.status.unwrap(),
            ProcessResult::Done(Val::string("Hello world")),
            "recv_req returns the request on connection w/ request id and contents"
        );
    }

    #[tokio::test]
    async fn recv_req_peval_send_resp() {
        let (local, mut remote) = Connection::pair().unwrap();
        let mut procs = ProcessSet::new();

        let prog = Program::from_expr("(send_resp (peval (recv_req)))").unwrap();
        let _ = Process::from_prog(0.into(), prog)
            .conn(local)
            .spawn(&mut procs);

        let _ = remote
            .send_req(Request {
                req_id: 10,
                contents: Form::string("Hello world"),
            })
            .await;
        let resp = remote.recv_resp().await;

        let res = procs.join_next().await.unwrap().unwrap();
        assert_eq!(res.status.unwrap(), ProcessResult::Done(Val::keyword("ok")),);
        assert_matches!(
            resp,
            Some(Ok(r)) if r.req_id == 10 && r.contents == Ok(Form::string("Hello world"))
        );
    }

    #[tokio::test]
    async fn get_self() {
        let mut procs = ProcessSet::new();

        let prog = Program::from_expr("(self)").unwrap();
        let _ = Process::from_prog(99.into(), prog).spawn(&mut procs);

        let res = procs.join_next().await.unwrap().unwrap();
        assert_eq!(
            res.status.unwrap(),
            ProcessResult::Done(Val::Extern(Extern::ProcessId(99.into())))
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
    // TODO: Test spawning invalid expressions - quote w/o any expressions
    // TODO: Test that dropping process handle ends process
    // TODO: Test ProcessHandle::kill
    // TODO: Test `spawn` builtin (w/ correct capture / eval semantics)
}
