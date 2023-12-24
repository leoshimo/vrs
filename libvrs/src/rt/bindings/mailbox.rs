//! Bindings for Process Mailbox
use crate::rt::{
    mailbox::Message,
    program::{Extern, Fiber, Lambda, NativeAsyncFn, Pattern, Val},
};
use lyric::{compile, parse, Error, Result, SymbolId};

pub(crate) fn send_fn() -> NativeAsyncFn {
    NativeAsyncFn {
        func: |f, args| Box::new(send_impl(f, args)),
    }
}

/// Binding to recv messages
pub(crate) fn recv_fn() -> NativeAsyncFn {
    NativeAsyncFn {
        func: |f, args| Box::new(recv_impl(f, args)),
    }
}

/// Binding to list messages
pub(crate) fn ls_msgs_fn() -> NativeAsyncFn {
    NativeAsyncFn {
        func: |f, args| Box::new(ls_msgs_impl(f, args)),
    }
}

/// Binding for call
pub(crate) fn call_fn() -> Lambda {
    Lambda {
        params: vec![SymbolId::from("pid"), SymbolId::from("msg")],
        code: compile(
            &parse(
                r#"
            (begin
                (def r (ref))
                (send pid (list r (self) msg))
                (get (recv (list r 'any)) 1))
        "#,
            )
            .unwrap()
            .into(),
        )
        .unwrap(),
        parent: None,
    }
}

/// Implementation for (send PID MSG)
async fn send_impl(fiber: &mut Fiber, args: Vec<Val>) -> Result<Val> {
    let src = fiber.locals().pid;
    let (dst, msg) = match &args[..] {
        [Val::Extern(Extern::ProcessId(dst)), msg] => (dst, msg),
        _ => {
            return Err(Error::UnexpectedArguments(
                "Unexpected send call - (send DEST_PID DATA)".to_string(),
            ))
        }
    };

    if src == *dst {
        fiber
            .locals()
            .self_handle
            .as_ref()
            .expect("process should have self handle")
            .notify_message(Message::new(src, msg.clone()))
            .await;
    } else {
        let kernel = fiber
            .locals()
            .kernel
            .as_ref()
            .and_then(|k| k.upgrade())
            .ok_or(Error::Runtime("Kernel is missing for process".to_string()))?;
        kernel
            .send_message(src, *dst, msg.clone())
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
        _ => {
            return Err(Error::UnexpectedArguments(
                "recv expects one or no arguments - (recv [PATTERN])".to_string(),
            ))
        }
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

/// Implementation for (ls-msgs)
async fn ls_msgs_impl(fiber: &mut Fiber, args: Vec<Val>) -> Result<Val> {
    if !args.is_empty() {
        return Err(Error::UnexpectedArguments(
            "Unexpected ls-msgs call - No arguments expected".to_string(),
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
        let k = kernel::start();

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
        let k = kernel::start();

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
                Val::Extern(Extern::ProcessId(send_pid))
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
    async fn ls_msgs_empty() {
        let k = kernel::start();

        let hdl = k
            .spawn_prog(Program::from_expr("(ls-msgs)").unwrap())
            .await
            .unwrap();

        let exit = hdl.join().await.unwrap();
        assert_eq!(exit.status.unwrap(), ProcessResult::Done(Val::List(vec![])))
    }

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn ls_msgs_nonempty() {
        let k = kernel::start();

        let hdl = k
            .spawn_prog(
                Program::from_expr(
                    "(begin
                        (send (self) :one)
                        (send (self) :two)
                        (send (self) :three)
                        (ls-msgs))",
                )
                .unwrap(),
            )
            .await
            .unwrap();

        let exit = hdl.join().await.unwrap();
        assert_eq!(
            exit.status.unwrap(),
            ProcessResult::Done(Val::from_expr("(:one :two :three)").unwrap()),
            "ls-msgs should contain all messages in order"
        );
    }

    #[tokio::test]
    async fn recv_with_pattern() {
        let k = kernel::start();

        let recv = k
            .spawn_prog(
                Program::from_expr(
                    "(begin
                (def match (recv :target))
                (list match (ls-msgs)))",
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
            "(recv :target) should return :target for first element, ls-msgs should return all ignored messages"
        );
    }

    #[tokio::test]
    async fn recv_with_pattern_nested() {
        let k = kernel::start();

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
}
