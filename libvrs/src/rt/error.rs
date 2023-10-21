use tokio::sync::oneshot;

use super::{kernel, process};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Failed to message kernel task - {0}")]
    FailedToMessageKernelTask(#[from] tokio::sync::mpsc::error::SendError<kernel::Message>),

    #[error("Failed to receive response from kernel task - {0}")]
    FailedToReceiveResponseFromKernelTask(tokio::sync::oneshot::error::RecvError),

    #[error("Failed to receive response from process event loop - {0}")]
    FailedToReceiveResponseFromProcessTask(tokio::sync::oneshot::error::RecvError),

    #[error("Failed to message process - {0}")]
    FailedToMessageProcess(#[from] tokio::sync::mpsc::error::SendError<process::Message>),

    #[error("Received unexpected process result")]
    UnexpectedProcessResult,

    #[error("Evaluation Error - {0}")]
    EvaluationError(#[from] lemma::Error),

    #[error("Process Exec Error - {0}")]
    ProcessExecError(lemma::Error),

    #[error("IO failed to message process IO task")]
    IOFailed,

    #[error("Failed to join process")]
    ProcessJoinError(#[from] oneshot::error::RecvError),
}
