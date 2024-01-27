//! Bindings for Process Mailbox
use crate::rt::{
    mailbox::Message,
    program::{Extern, Fiber, NativeAsyncFn, Pattern, Val},
};
use lyric::{Error, Result, SymbolId};

pub(crate) fn send_fn() -> NativeAsyncFn {
    NativeAsyncFn {
        doc: "(send PID MSG) - Send process PID the message MSG".to_string(),
        func: |f, args| Box::new(send_impl(f, args)),
    }
}

/// Binding to recv messages
pub(crate) fn recv_fn() -> NativeAsyncFn {
    NativeAsyncFn {
        doc: "(recv [PATTERN]) - Poll mailbox for a message. \
              Optional PATTERN argument can match for messages matching specific patterns."
            .to_string(),
        func: |f, args| Box::new(recv_impl(f, args)),
    }
}

/// Binding to list messages
pub(crate) fn ls_msgs_fn() -> NativeAsyncFn {
    NativeAsyncFn {
        doc: "(ls_msgs) - Returns contents of mailbox without consuming messages or blocking when mailbox is empty.".to_string(),
        func: |f, args| Box::new(ls_msgs_impl(f, args)),
    }
}

/// Binding for call
pub(crate) fn call_fn() -> NativeAsyncFn {
    NativeAsyncFn {
        doc: "(call PID MSG) - Send a request and wait for its response, up to the calling process's timeout".to_string(),
        func: |fiber, args| Box::new(call_impl(fiber, args)),
    }
}

async fn call_impl(fiber: &mut Fiber, args: Vec<Val>) -> Result<Val> {
    let (pid, msg, timeout) = match args.as_slice() {
        [Val::Extern(Extern::ProcessId(pid)), msg] => {
            (pid.clone(), msg.clone(), fiber.locals().call_timeout)
        }
        _ => {
            return Err(Error::UnexpectedArguments(
                "call expects a process id and message".to_string(),
            ))
        }
    };

    let request_ref = lyric::Ref::unique();
    let request = Val::List(vec![
        Val::Ref(request_ref.clone()),
        Val::Extern(Extern::ProcessId(fiber.locals().pid.clone())),
        msg,
    ]);
    send_impl(
        fiber,
        vec![Val::Extern(Extern::ProcessId(pid.clone())), request],
    )
    .await?;

    let pattern = Pattern::from_val(Val::List(vec![
        Val::Ref(request_ref),
        Val::Symbol(SymbolId::from("response")),
    ]));
    let mailbox = fiber
        .locals()
        .self_handle
        .as_ref()
        .expect("process should have self handle")
        .mailbox()
        .clone();
    let response = mailbox
        .poll_timeout(Some(pattern), timeout)
        .await
        .map_err(|e| Error::Runtime(format!("{e}")))?
        .ok_or_else(|| {
            Error::Runtime(format!(
                "call to {pid} timed out after {} seconds",
                timeout.as_secs()
            ))
        })?;
    response
        .contents
        .as_list()?
        .get(1)
        .cloned()
        .ok_or_else(|| Error::Runtime("call received a malformed response".to_string()))
}

/// Implementation for (send PID MSG)
async fn send_impl(fiber: &mut Fiber, args: Vec<Val>) -> Result<Val> {
    let src = fiber.locals().pid.clone();
    let (dst, msg) = match &args[..] {
        [Val::Extern(Extern::ProcessId(dst)), msg] => (dst.clone(), msg),
        _ => {
            return Err(Error::UnexpectedArguments(
                "Unexpected send call - (send DEST_PID DATA)".to_string(),
            ))
        }
    };

    if dst == src {
        fiber
            .locals()
            .self_handle
            .as_ref()
            .expect("process should have self handle")
            .notify_message(Message::new(msg.clone()))
            .await;
    } else if dst.node() == fiber.locals().node_name {
        let kernel = fiber
            .locals()
            .kernel
            .as_ref()
            .and_then(|k| k.upgrade())
            .ok_or(Error::Runtime("Kernel is missing for process".to_string()))?;
        kernel
            .send_message(dst, msg.clone())
            .await
            .map_err(|e| Error::Runtime(format!("{e}")))?;
    } else {
        let peers = fiber
            .locals()
            .peers
            .as_ref()
            .ok_or_else(|| Error::Runtime("Node links are not available".to_string()))?;
        peers
            .route(dst, msg.clone())
            .await
            .map_err(|e| Error::Runtime(format!("{e}")))?;
    }

    Ok(msg.clone())
}

/// Implementation for (recv PAT)
async fn recv_impl(fiber: &mut Fiber, args: Vec<Val>) -> Result<Val> {
    let pat = match &args[..] {
        [pat] => Some(Pattern::from_val(pat.clone())),
        [] => None,
        _ => Some(Pattern::from_vals(&args[..])),
    };
    let mailbox = fiber
        .locals()
        .self_handle
        .as_ref()
        .expect("process should have self handle")
        .mailbox();
    let msg = mailbox
        .poll(pat)
        .await
        .map_err(|e| Error::Runtime(format!("{e}")))?;
    Ok(msg.contents)
}

/// Implementation for (ls_msgs)
async fn ls_msgs_impl(fiber: &mut Fiber, args: Vec<Val>) -> Result<Val> {
    if !args.is_empty() {
        return Err(Error::UnexpectedArguments(
            "Unexpected ls_msgs call - No arguments expected".to_string(),
        ));
    }

    let mailbox = fiber
        .locals()
        .self_handle
        .as_ref()
        .expect("process should have self handle")
        .mailbox();

    let msgs = mailbox
        .all()
        .await
        .map_err(|e| Error::Runtime(format!("{e}")))?;
    let msg_vals = msgs.into_iter().map(|m| m.contents).collect();

    Ok(Val::List(msg_vals))
}

#[cfg(test)]
mod tests {

    use crate::rt::{kernel, ProcessResult};
    use crate::{Program, Val};

    use super::*;

    #[tokio::test]
    async fn send_recv_one() {
        let k = kernel::start_test();

        let hdl = k
            .spawn_prog(
                Program::from_expr(
                    r#"(begin
                        (send (self) :hello_from_self)
                        (recv))"#,
                )
                .unwrap(),
            )
            .await
            .unwrap();

        let exit = hdl.join().await.unwrap();
        assert_eq!(
            exit.status.unwrap(),
            ProcessResult::Done(Val::keyword("hello_from_self"))
        );
    }

    #[tokio::test]
    async fn send_recv_two() {
        let k = kernel::start_test();

        let recv = k
            .spawn_prog(Program::from_expr("(recv)").unwrap())
            .await
            .unwrap();

        let send = k
            .spawn_prog(
                Program::from_expr(
                    format!("(send (pid {}) (list :hi :from (self)))", recv.id().inner()).as_str(),
                )
                .unwrap(),
            )
            .await
            .unwrap();

        let send_pid = send.id();
        assert_eq!(
            send.join().await.unwrap().status.unwrap(),
            ProcessResult::Done(Val::List(vec![
                Val::keyword("hi"),
                Val::keyword("from"),
                Val::Extern(Extern::ProcessId(send_pid.clone()))
            ])),
            "send should return sent message"
        );

        assert_eq!(
            recv.join().await.unwrap().status.unwrap(),
            ProcessResult::Done(Val::List(vec![
                Val::keyword("hi"),
                Val::keyword("from"),
                Val::Extern(Extern::ProcessId(send_pid))
            ])),
            "recv should receive message"
        );
    }

    #[tokio::test]
    async fn call_timeout_does_not_prevent_the_caller_from_continuing() {
        let k = kernel::start_test();
        let hdl = k
            .spawn_prog(
                Program::from_expr(
                    r#"(begin
                        (call_timeout 0)
                        (def target (spawn (lambda () (recv))))
                        (def timeout_error (try (call target :hello)))
                        (send (self) :continued)
                        (list timeout_error (recv)))"#,
                )
                .unwrap(),
            )
            .await
            .unwrap();

        let result = hdl.join().await.unwrap().status.unwrap();
        let ProcessResult::Done(Val::List(values)) = result else {
            panic!("call should time out and let the process continue");
        };
        assert!(matches!(
            &values[..],
            [Val::Error(lyric::Error::Runtime(message)), Val::Keyword(continued)]
                if message.contains("timed out after 0 seconds") && continued.as_str() == "continued"
        ));
    }

    #[tokio::test]
    async fn ls_msgs_empty() {
        let k = kernel::start_test();

        let hdl = k
            .spawn_prog(Program::from_expr("(ls_msgs)").unwrap())
            .await
            .unwrap();

        let exit = hdl.join().await.unwrap();
        assert_eq!(exit.status.unwrap(), ProcessResult::Done(Val::List(vec![])))
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn ls_msgs_nonempty() {
        let k = kernel::start_test();

        let hdl = k
            .spawn_prog(
                Program::from_expr(
                    "(begin
                        (send (self) :one)
                        (send (self) :two)
                        (send (self) :three)
                        (ls_msgs))",
                )
                .unwrap(),
            )
            .await
            .unwrap();

        let exit = hdl.join().await.unwrap();
        assert_eq!(
            exit.status.unwrap(),
            ProcessResult::Done(Val::from_expr("(:one :two :three)").unwrap()),
            "ls_msgs should contain all messages in order"
        );
    }

    #[tokio::test]
    async fn recv_with_pattern() {
        let k = kernel::start_test();

        let recv = k
            .spawn_prog(
                Program::from_expr(
                    "(begin
                (def match (recv :target))
                (list match (ls_msgs)))",
                )
                .unwrap(),
            )
            .await
            .unwrap();

        let send = k
            .spawn_prog(
                Program::from_expr(&format!(
                    r#"(begin
                        (def other (pid {}))
                        (send other :ignored_one)
                        (send other :ignored_two)
                        (send other '(:target :ignored_three))
                        (send other :target))"#,
                    recv.id().inner()
                ))
                .unwrap(),
            )
            .await
            .unwrap();

        let _ = send.join().await.unwrap();

        let exit = recv.join().await.unwrap();
        assert_eq!(
            exit.status.unwrap(),
            ProcessResult::Done(
                Val::from_expr("(
                    :target
                    (:ignored_one :ignored_two (:target :ignored_three))
                )").unwrap()
            ),
            "(recv :target) should return :target for first element, ls_msgs should return all ignored messages"
        );
    }

    #[tokio::test]
    async fn recv_with_pattern_nested() {
        let k = kernel::start_test();

        let recv = k
            .spawn_prog(
                Program::from_expr(
                    "(list (recv '(:one :two three))
                           (recv '(:four (five) ((six)))))",
                )
                .unwrap(),
            )
            .await
            .unwrap();

        let send = k
            .spawn_prog(
                Program::from_expr(&format!(
                    r#"(begin
                        (def other (pid {}))
                        (send other :ignored)
                        (send other '(:one :one 1))
                        (send other '(:one :two 3))
                        (send other '(:four 5 6))
                        (send other '(:four (5) ((6)))))"#,
                    recv.id().inner()
                ))
                .unwrap(),
            )
            .await
            .unwrap();

        let _ = send.join().await.unwrap();

        let exit = recv.join().await.unwrap();
        assert_eq!(
            exit.status.unwrap(),
            ProcessResult::Done(
                Val::from_expr("(
                    (:one :two 3)
                    (:four (5) ((6)))
                )").unwrap()
            ),
            "(recv '(:one :two three)) should match (:one :two 3), (recv '(:four (five) ((six)))) should match (:four (5) ((6)))"
        );
    }

    #[tokio::test]
    async fn recv_with_multipattern() {
        let k = kernel::start_test();

        let prog = r#"(begin
            (send (self) :one)
            (send (self) :ignore)
            (send (self) '(:two 2))
            (send (self) '(:three :ignore))
            (send (self) '(:four 4 5)) # ignored
            (send (self) '(:four 4 4))
            (send (self) '(:five 5 5 :ignore))
            (list
                  (recv :one
                        '(:two _)
                        '(:three 3)
                        '(four a a)
                        '(:five _ _))
                  (recv :one
                        '(:two _)
                        '(:three 3)
                        '(four a a)
                        '(:five _ _))
                  (recv :one
                        '(:two _)
                        '(:three 3)
                        '(four a a)
                        '(:five _ _))
            ))
        "#;
        let hdl = k
            .spawn_prog(Program::from_expr(prog).unwrap())
            .await
            .unwrap();

        let exit = hdl.join().await.unwrap();
        assert_eq!(
            exit.status.unwrap(),
            ProcessResult::Done(
                Val::from_expr(
                    "(:one
                      (:two 2)
                      (:four 4 4))"
                )
                .unwrap()
            )
        );
    }
}
