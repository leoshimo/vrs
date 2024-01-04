//! Bindings for interacting with [Connection]
use crate::rt::program::{Fiber, NativeFn, NativeFnOp, Val};
use lyric::Result;

/// Binding for `recv_req` to receive requests over client connection
pub(crate) fn recv_req_fn() -> NativeFn {
    NativeFn {
        func: |_, _| {
            Ok(lyric::NativeFnOp::Return(lyric::Val::Int(0)))
            // Ok(NativeFnOp::Yield(Val::Extern(Extern::IOCmd(Box::new(
            //     IOCmd::RecvRequest,
            // )))))
        },
    }
}

/// Binding for `send_resp` to send responses over client connection
pub(crate) fn send_resp_fn() -> NativeFn {
    NativeFn { func: send_resp }
}

/// Implements `send_resp`
fn send_resp(_f: &mut Fiber, _args: &[Val]) -> Result<NativeFnOp> {
    Ok(lyric::NativeFnOp::Return(lyric::Val::Int(0)))
    // let val = match args {
    //     [v] => v.clone(),
    //     _ => {
    //         return Err(Error::UnexpectedArguments(
    //             "send_conn expects two arguments".to_string(),
    //         ))
    //     }
    // };
    // Ok(NativeFnOp::Yield(Val::Extern(Extern::IOCmd(Box::new(
    //     IOCmd::SendResponse(val),
    // )))))
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
