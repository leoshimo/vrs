//! Bindings for interacting with [Connection]
use crate::{
    rt::program::{Extern, Fiber, NativeAsyncFn, Val},
    Response,
};
use lyric::{Error, Result};

/// Binding for `recv_req` to receive requests over client connection
pub(crate) fn recv_req_fn() -> NativeAsyncFn {
    NativeAsyncFn {
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

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::rt::program::Form;
//     use crate::rt::{Process, ProcessSet};
//     use crate::{Connection, ProcessResult, Program, Request};
//     use assert_matches::assert_matches;

// #[ignore]
// #[tokio::test]
// async fn recv_req() {
//     // let (local, mut remote) = Connection::pair().unwrap();

//     let mut procs = ProcessSet::new();
//     let prog = Program::from_expr("(recv_req)").unwrap();
//     let _ = Process::from_prog(0.into(), prog)
//         // .conn(local)
//         .spawn(&mut procs);

//     let _ = remote
//         .send_req(Request {
//             id: 0,
//             contents: Form::string("Hello world"),
//         })
//         .await;

//     let res = procs.join_next().await.unwrap().unwrap();
//     assert_eq!(
//         res.status.unwrap(),
//         ProcessResult::Done(Val::string("Hello world")),
//         "recv_req returns the request on connection w/ request id and contents"
//     );
// }

// #[ignore]
// #[tokio::test]
// async fn recv_req_try_eval_send_resp() {
//     // let (local, mut remote) = Connection::pair().unwrap();
//     let mut procs = ProcessSet::new();

//     let prog = Program::from_expr("(send_resp (try (eval (recv_req))))").unwrap();
//     let _ = Process::from_prog(0.into(), prog)
//         // .conn(local)
//         .spawn(&mut procs);

//     let _ = remote
//         .send_req(Request {
//             id: 10,
//             contents: Form::string("Hello world"),
//         })
//         .await;
//     let resp = remote.recv_resp().await;

//     let res = procs.join_next().await.unwrap().unwrap();
//     assert_eq!(res.status.unwrap(), ProcessResult::Done(Val::keyword("ok")),);
//     assert_matches!(
//         resp,
//         Some(Ok(r)) if r.req_id == 10 && r.contents == Ok(Form::string("Hello world"))
//     );
// }
// }

// TODO: Test that term bindings are not available for standard processes
