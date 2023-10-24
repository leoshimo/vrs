use super::proc_bindings;
use super::proc_io::{IOCmd, ProcIO};
use crate::rt::{Error, Result};
use crate::Connection;
use lyric::{parse, FiberState};
use tokio::sync::mpsc;
use tokio::task::JoinSet;

/// Set of running processes
pub type ProcessSet = JoinSet<ProcessExit>;

/// IDs assigned to processes
pub type ProcessId = usize;

/// A running process in runtime
pub struct Process {
    id: ProcessId,
    fiber: Fiber,
    io: ProcIO,
}

/// Values produced by processes
pub type Val = lyric::Val<Extern, ()>;

/// Fibers for processes
pub type Fiber = lyric::Fiber<Extern, ()>;

/// NativeFn type for Process bindings
pub type NativeFn = lyric::NativeFn<Extern, ()>;

/// NativeFnVal for Process
pub type NativeFnVal = lyric::NativeFnVal<Extern, ()>;

/// A handle to [Process]
#[derive(Debug)]
pub struct ProcessHandle {
    msg_tx: mpsc::Sender<Message>,
}

/// Extern type between Fiber and hosting Process
#[derive(Debug, Clone, PartialEq)]
pub enum Extern {
    /// IO Commands
    IOCmd(Box<IOCmd>),
}

/// The result of process
#[derive(Debug, Clone, PartialEq)]
pub enum ProcessResult {
    /// Completed with value
    Done(Val),
    /// Cancelled for closed event loop
    Cancelled,
}

/// A record of process exiting
#[derive(Debug)]
pub struct ProcessExit {
    pub id: ProcessId,
    pub status: Result<ProcessResult>,
}

impl Process {
    /// Create a new process from val
    pub fn from_val(id: ProcessId, val: Val) -> Result<Self> {
        let mut fiber = Fiber::from_val(&val, ())?;
        fiber
            .bind(proc_bindings::recv_req_fn())
            .bind(proc_bindings::send_resp_fn());
        Ok(Self {
            id,
            fiber,
            io: ProcIO::new(),
        })
    }

    /// Create a new process from expression
    pub fn from_expr(id: ProcessId, expr: &str) -> Result<Self> {
        Self::from_val(id, parse(expr)?.into())
    }

    /// Set connection on process
    pub fn conn(mut self, conn: Connection) -> Self {
        self.io.conn(conn);
        self
    }

    /// Spawn a process
    pub fn spawn(self, procs: &mut ProcessSet) -> Result<ProcessHandle> {
        let (msg_tx, mut msg_rx) = mpsc::channel(32);
        procs.spawn(async move {
            let exit: Result<_> = async {
                let mut fiber = self.fiber;
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
                            tokio::select!(
                                Some(msg) = msg_rx.recv() => match msg {
                                    Message::Kill => return Ok(ProcessExit {
                                        id: self.id,
                                        status: Ok(ProcessResult::Cancelled)
                                    })
                                },
                                io_result = Self::handle_yield(v, &mut io) => {
                                    let io_result = io_result?;
                                    state = fiber.resume_from_yield(io_result)?;
                                }
                            );
                        }
                    }
                }
            }
            .await;
            match exit {
                Ok(exit) => exit,
                Err(e) => ProcessExit {
                    id: self.id,
                    status: Err(e),
                },
            }
        });
        Ok(ProcessHandle { msg_tx })
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
    /// Kill process. Effect is not immediate.
    pub async fn kill(&self) {
        let _ = self.msg_tx.send(Message::Kill).await;
    }
}

#[derive(Debug)]
enum Message {
    Kill,
}

impl std::fmt::Display for Extern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<signal>")
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
        let _ = Process::from_expr(99, "\"Hello\"")
            .unwrap()
            .spawn(&mut procs)
            .unwrap();

        let res = procs.join_next().await.unwrap().unwrap();
        assert_eq!(res.id, 99,);
        assert_eq!(
            res.status.unwrap(),
            ProcessResult::Done(Val::string("Hello")),
        );
    }

    #[tokio::test]
    async fn processes_are_isolated() {
        let mut procs = ProcessSet::new();
        let _ = Process::from_expr(0, "(def x 0)")
            .unwrap()
            .spawn(&mut procs)
            .unwrap();
        let res = procs.join_next().await.unwrap().unwrap();
        assert_eq!(res.status.unwrap(), ProcessResult::Done(Val::Int(0)),);

        let _ = Process::from_expr(0, "x")
            .unwrap()
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
        let _ = Process::from_expr(0, "(recv_req)")
            .unwrap()
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

        let _ = Process::from_expr(0, "(send_resp (peval (recv_req)))")
            .unwrap()
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
        assert_eq!(res.status.unwrap(), ProcessResult::Done(Val::symbol("ok")),);
        assert_matches!(
            resp,
            Some(Ok(r)) if r.req_id == 10 && r.contents == Ok(Form::string("Hello world"))
        );
    }

    // TODO: Implement + test preemption
    // #[tokio::test]
    // async fn drop_handle_ends_process() {
    //     let mut procs = ProcessSet::new();

    //     let handle = spawn(parse("(loop 0)").unwrap().into(), &mut procs).unwrap();
    //     drop(handle)l

    //     assert_eq!(procs.join_next().await.unwrap().unwrap().unwrap(),
    //                ProcessResult::Cancelled);
    // }

    // TODO: Test that top-level yield of jibberish like (yield 1) results in process terminating w/ error
    // TODO: Test spawning invalid expressions - quote w/o any expressions
    // TODO: Test that dropping process handle ends process
    // TODO: Test ProcessHandle::kill
}
