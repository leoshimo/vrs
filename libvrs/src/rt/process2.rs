#![allow(dead_code)]
use crate::rt::{Error, Result};
use lemma::FiberState;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinSet;

pub(crate) type ProcessSet = JoinSet<Result<Val>>;

/// Handle to process
#[derive(Debug)]
pub struct ProcessHandle {
    msg_tx: mpsc::Sender<Message>,
}

/// Values produced by processes
pub type Val = lemma::Val<Signal>;

/// Signal type between Fiber and hosting Process
#[derive(Debug, Clone, PartialEq)]
pub enum Signal {}

/// Spawn a new process
pub fn spawn(prog: Val, procs: &mut ProcessSet) -> Result<ProcessHandle> {
    let (msg_tx, _msg_rx) = mpsc::channel(32);

    procs.spawn(async move {
        let mut fiber = Fiber::from_val(&prog)?;
        let state = fiber.resume()?;
        let val = match state {
            FiberState::Done(val) => val,
            FiberState::Yield(_) => todo!(),
        };
        Ok::<Val, Error>(val)
    });

    Ok(ProcessHandle { msg_tx })
}

/// Running process in runtime
struct Process {
    /// Executing fiber of process
    fiber: Fiber,
}

type Fiber = lemma::Fiber<Signal>;

/// Messages driving process execution
enum Message {}

impl std::fmt::Display for Signal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<signal>")
    }
}

#[cfg(test)]
mod tests {

    use assert_matches::assert_matches;
    use lemma::parse;

    use super::*;

    #[tokio::test]
    async fn spawn_simple() {
        let mut procs = ProcessSet::new();
        let _ = spawn(Val::string("Hello"), &mut procs).unwrap();
        assert_eq!(
            procs.join_next().await.unwrap().unwrap().unwrap(),
            Val::string("Hello"),
        );
    }

    #[tokio::test]
    async fn processes_are_isolated() {
        let mut procs = ProcessSet::new();

        let _ = spawn(parse("(def x 0)").unwrap().into(), &mut procs).unwrap();
        assert_eq!(
            procs.join_next().await.unwrap().unwrap().unwrap(),
            Val::Int(0),
        );

        let _ = spawn(parse("x").unwrap().into(), &mut procs).unwrap();
        assert_matches!(
            procs.join_next().await.unwrap().unwrap(),
            Err(Error::EvaluationError(lemma::Error::UndefinedSymbol(_))),
            "processes should not share environment by default",
        );
    }

    // TODO: Test spawning invalid expressions - quote w/o any expressions
}
