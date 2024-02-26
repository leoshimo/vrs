//! Bindings for interacting with [Connection]
use crate::{
    rt::program::{Extern, Fiber, NativeAsyncFn, Val},
    Response,
};
use lyric::{Error, Result};

/// Binding for `recv_req` to receive requests over client connection
pub(crate) fn recv_req_fn() -> NativeAsyncFn {
    NativeAsyncFn {
        doc: "(recv_req) - Receive request over client connection. This blocks until request is received.".to_string(),
        func: |f, _| Box::new(recv_req_impl(f)),
    }
}

/// Implementation of (RECV_REQ)
async fn recv_req_impl(fiber: &mut Fiber) -> Result<Val> {
    let term = fiber.locals().term.as_ref().ok_or(Error::Runtime(
        "recv_req failed - no connected terminal".to_string(),
    ))?;

    let req = term
        .read_request()
        .await
        .map_err(|e| Error::Runtime(format!("recv_req error - {e}")))?;

    Ok(Val::List(vec![
        Val::Extern(Extern::RequestId(req.id)),
        req.contents.into(),
    ]))
}

/// Binding for `send_resp` to send responses over client connection
pub(crate) fn send_resp_fn() -> NativeAsyncFn {
    NativeAsyncFn {
        doc: "(send_resp REQ_ID RESP) - Send response RESP over client connection for request identified by REQ_ID. This blocks until response is sent.".to_string(),
        func: |f, args| Box::new(send_resp_impl(f, args)),
    }
}

/// Implements `send_resp`
async fn send_resp_impl(fiber: &mut Fiber, args: Vec<Val>) -> Result<Val> {
    let (req_id, contents) = match &args[..] {
        [Val::Extern(Extern::RequestId(req_id)), c] => (req_id, c),
        _ => {
            return Err(Error::UnexpectedArguments(
                "send_conn expects two arguments".to_string(),
            ))
        }
    };

    let term = fiber.locals().term.as_ref().ok_or(Error::Runtime(
        "recv_req failed - no connected terminal".to_string(),
    ))?;

    let resp = Response {
        req_id: *req_id,
        contents: Ok(contents
            .clone()
            .try_into()
            .map_err(|e| Error::Runtime(format!("{e}")))?),
    };

    term.send_response(resp)
        .await
        .map_err(|e| Error::Runtime(format!("{e}")))?;

    Ok(Val::keyword("ok"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rt::program::{self, term_env, Form};
    use crate::rt::pubsub::PubSub;
    use crate::rt::term::Term;
    use crate::rt::{kernel, Process, ProcessSet};
    use crate::{Connection, ProcessResult, Program, Request};
    use assert_matches::assert_matches;

    #[tokio::test]
    async fn recv_req() {
        let (local, mut remote) = Connection::pair().unwrap();

        let mut procs = ProcessSet::new();
        let prog = Program::from_expr("(recv_req)").unwrap().env(term_env());
        let _ = Process::from_prog(0.into(), prog)
            .term(Term::spawn(local, PubSub::spawn()))
            .spawn(&mut procs);

        let _ = remote
            .send_req(Request {
                id: 2,
                contents: Form::string("Hello world"),
            })
            .await;

        let res = procs.join_next().await.unwrap().unwrap();
        assert_eq!(
            res.status.unwrap(),
            ProcessResult::Done(Val::List(vec![
                Val::Extern(Extern::RequestId(2)),
                Val::string("Hello world"),
            ])),
            "recv_req returns request id and contents"
        );
    }

    #[tokio::test]
    async fn send_resp() {
        let (local, mut remote) = Connection::pair().unwrap();
        let mut procs = ProcessSet::new();

        let prog = r#"(begin
            # Echo single response
            (def (req_id contents) (recv_req))
            (send_resp req_id "Goodbye world"))
        "#;
        let prog = Program::from_expr(prog).unwrap().env(term_env());
        let hdl = Process::from_prog(0.into(), prog)
            .term(Term::spawn(local, PubSub::spawn()))
            .spawn(&mut procs)
            .unwrap();

        let _ = remote
            .send_req(Request {
                id: 2,
                contents: Form::string("Hello world"),
            })
            .await;

        let res = hdl.join().await.unwrap();
        assert_eq!(res.status.unwrap(), ProcessResult::Done(Val::keyword("ok")),);

        let resp = remote
            .recv_resp()
            .await
            .expect("should be some")
            .expect("should be ok");
        assert_eq!(
            resp,
            Response {
                req_id: 2,
                contents: Ok(Form::string("Goodbye world"))
            }
        );
    }

    #[tokio::test]
    async fn term_e2e() {
        let (local, mut remote) = Connection::pair().unwrap();
        let mut procs = ProcessSet::new();

        let prog = program::term_prog();
        let _ = Process::from_prog(0.into(), prog)
            .term(Term::spawn(local, PubSub::spawn()))
            .spawn(&mut procs);

        let _ = remote
            .send_req(Request {
                id: 10,
                contents: Val::from_expr("(+ 1 2)").unwrap().try_into().unwrap(),
            })
            .await;
        let resp = remote.recv_resp().await;

        assert_matches!(
            resp,
            Some(Ok(r)) if r.req_id == 10 && r.contents == Ok(Form::Int(3))
        );
    }

    #[tokio::test]
    async fn standard_procs_has_no_bindings() {
        let k = kernel::start();

        {
            let prog = Program::from_expr("(recv_req)").unwrap().env(term_env());
            let proc = k.spawn_prog(prog).await.unwrap();
            let res = proc.join().await;
            assert_matches!(res, Ok(_));
        }
    }

    // TODO: Test cases for term bindings w/o Term handle (via missing or term crash)
}
