#![allow(dead_code)]
use crate::rt::{Error, Result};
use crate::{Connection, Response};
use lyric::{FiberState, Form, NativeFn, NativeFnVal, SymbolId};
use tokio::sync::mpsc;
use tokio::task::JoinSet;

pub(crate) type ProcessSet = JoinSet<Result<ProcessResult>>;

/// Handle to process
#[derive(Debug)]
pub struct ProcessHandle {
    msg_tx: mpsc::Sender<Message>,
}

#[derive(Debug)]
enum Message {
    Kill,
}

/// Values produced by processes
pub type Val = lyric::Val<Extern>;

/// Extern type between Fiber and hosting Process
#[derive(Debug, Clone, PartialEq)]
pub enum Extern {
    /// IO Commands
    IOCmd(Box<IOCmd>),
}

/// IO Commands that can be yielded from fiber
#[derive(Debug, Clone, PartialEq)]
pub enum IOCmd {
    /// Signal to notify for RecvConn IO
    RecvConn,
    /// Send value over connection for given request ID
    SendConn(i32, lyric::Val<Extern>),
}

/// The result of process
#[derive(Debug, Clone, PartialEq)]
pub enum ProcessResult {
    /// Completed with value
    Done(Val),
    /// Cancelled for closed event loop
    Cancelled,
}

impl ProcessHandle {
    /// Kill process. Effect is not immediate.
    pub async fn kill(&self) {
        let _ = self.msg_tx.send(Message::Kill).await;
    }
}

/// Spawn a new process
pub fn spawn(prog: Val, procs: &mut ProcessSet, conn: Option<Connection>) -> Result<ProcessHandle> {
    let (msg_tx, mut msg_rx) = mpsc::channel(32);
    procs.spawn(async move {
        let mut fiber = create_fiber(&prog)?;
        let mut state = fiber.resume()?;
        let mut conn = conn;
        loop {
            match state {
                FiberState::Done(v) => return Ok(ProcessResult::Done(v)),
                FiberState::Yield(v) => {
                    tokio::select!(
                        Some(msg) = msg_rx.recv() => match msg {
                            Message::Kill => return Ok(ProcessResult::Cancelled),
                        },
                        io_result = handle_io(v, &mut conn) => {
                            let io_result = io_result?;
                            state = fiber.resume_from_yield(io_result)?;
                        }
                    );
                }
            }
        }
    });

    Ok(ProcessHandle { msg_tx })
}

fn create_fiber(prog: &Val) -> Result<Fiber> {
    let mut fiber = Fiber::from_val(prog)?;

    // TODO: Revisit request / response with recv_conn / send_conn
    fiber.bind(NativeFn {
        symbol: SymbolId::from("recv_conn"),
        func: |_, _| {
            Ok(NativeFnVal::Yield(Val::Extern(Extern::IOCmd(Box::new(
                IOCmd::RecvConn,
            )))))
        },
    });
    fiber.bind(NativeFn {
        symbol: SymbolId::from("send_conn"),
        func: |_, args| -> std::result::Result<NativeFnVal<Extern>, lyric::Error> {
            let (id, val) = match args {
                [Val::Int(id), v] => (id, v.clone()),
                _ => {
                    return Err(lyric::Error::InvalidExpression(
                        "send_conn expects two arguments".to_string(),
                    ))
                }
            };
            Ok(NativeFnVal::Yield(Val::Extern(Extern::IOCmd(Box::new(
                IOCmd::SendConn(*id, val),
            )))))
        },
    });

    Ok(fiber)
}

async fn handle_io(val: Val, conn: &mut Option<Connection>) -> Result<Val> {
    let iocmd = match val {
        Val::Extern(Extern::IOCmd(cmd)) => cmd,
        _ => return Err(Error::UnexpectedSignal),
    };

    // TODO: How should errors from handling IO commands be handled - native / fiber?
    match *iocmd {
        IOCmd::RecvConn => {
            let req = conn
                .as_mut()
                .ok_or(Error::ProcessIOError(
                    "No connection bound to processs".to_string(),
                ))?
                .recv_req()
                .await
                .ok_or(Error::ProcessIOError("Connection closed".to_string()))??;

            Ok(Val::List(vec![
                Val::Int(req.req_id as i32),
                req.contents.into(),
            ]))
        }
        IOCmd::SendConn(req_id, contents) => {
            let contents = match TryInto::<Form>::try_into(contents) {
                Ok(c) => c,
                Err(e) => return Ok(Val::Error(e)),
            };

            let resp = Response {
                req_id: req_id as u32,
                contents: Ok(contents),
            };

            let resp = conn
                .as_mut()
                .ok_or(Error::ProcessIOError(
                    "No connection bound to processs".to_string(),
                ))?
                .send_resp(resp)
                .await;
            match resp {
                Ok(()) => Ok(Val::symbol("ok")),
                Err(_) => Ok(Val::symbol("err")),
            }
        }
    }
}

type Fiber = lyric::Fiber<Extern>;

impl std::fmt::Display for Extern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<signal>")
    }
}

#[cfg(test)]
mod tests {

    use assert_matches::assert_matches;
    use lyric::{parse, Form};

    use crate::Request;

    use super::*;

    #[tokio::test]
    async fn spawn_simple() {
        let mut procs = ProcessSet::new();
        let _ = spawn(Val::string("Hello"), &mut procs, None).unwrap();
        assert_eq!(
            procs.join_next().await.unwrap().unwrap().unwrap(),
            ProcessResult::Done(Val::string("Hello")),
        );
    }

    #[tokio::test]
    async fn processes_are_isolated() {
        let mut procs = ProcessSet::new();

        let _ = spawn(parse("(def x 0)").unwrap().into(), &mut procs, None).unwrap();
        assert_eq!(
            procs.join_next().await.unwrap().unwrap().unwrap(),
            ProcessResult::Done(Val::Int(0)),
        );

        let _ = spawn(parse("x").unwrap().into(), &mut procs, None).unwrap();
        assert_matches!(
            procs.join_next().await.unwrap().unwrap(),
            Err(Error::EvaluationError(lyric::Error::UndefinedSymbol(_))),
            "processes should not share environment by default",
        );
    }

    #[tokio::test]
    async fn recv_conn() {
        let (local, mut remote) = Connection::pair().unwrap();

        let mut procs = ProcessSet::new();
        let _ = spawn(
            parse("(recv_conn)").unwrap().into(),
            &mut procs,
            Some(local),
        );

        let _ = remote
            .send_req(Request {
                req_id: 0,
                contents: Form::string("Hello world"),
            })
            .await;

        assert_eq!(
            procs.join_next().await.unwrap().unwrap().unwrap(),
            ProcessResult::Done(Val::List(vec![Val::Int(0), Val::string("Hello world")])),
            "recv_conn returns the request on connection w/ request id and contents"
        );
    }

    #[tokio::test]
    async fn send_conn() {
        let (local, mut remote) = Connection::pair().unwrap();
        let mut procs = ProcessSet::new();

        let _ = spawn(
            parse("(send_conn 10 \"Hello from process\")")
                .unwrap()
                .into(),
            &mut procs,
            Some(local),
        );

        let resp = remote.recv_resp().await;

        assert_eq!(
            procs.join_next().await.unwrap().unwrap().unwrap(),
            ProcessResult::Done(Val::symbol("ok")),
        );
        assert_matches!(
            resp,
            Some(Ok(r)) if r.req_id == 10 && r.contents == Ok(Form::string("Hello from process"))
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
