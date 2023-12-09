use tokio::sync::oneshot;

#[derive(Debug, thiserror::Error, Clone)]
pub enum Error {
    #[error("Failed to send message to task - {0}")]
    NoMessageReceiver(String),

    #[error("Failed to receive response from kernel task - {0}")]
    FailedToReceiveResponseFromKernelTask(tokio::sync::oneshot::error::RecvError),

    #[error("Failed to receive response from process event loop - {0}")]
    FailedToReceiveResponseFromProcessTask(tokio::sync::oneshot::error::RecvError),

    #[error("Received unexpected process result")]
    UnexpectedProcessResult,

    #[error("Evaluation Error - {0}")]
    EvaluationError(#[from] lyric::Error),

    #[error("Registry Error - {0}")]
    RegistryError(String),

    #[error("Process Exec Error - {0}")]
    ProcessExecError(lyric::Error),

    #[error("IO failed to message process IO task")]
    IOFailed,

    #[error("Connection closed")]
    ConnectionClosed,

    #[error("Failed to join process")]
    ProcessJoinError(#[from] oneshot::error::RecvError),

    #[error("Unexpected top-level yield")]
    UnexpectedYield,

    #[error("Process IO Error - {0}")]
    ProcessIOError(String),

    #[error("IO Error - {0}")]
    IOError(String),

    #[error("No kernel")]
    NoKernel,

    #[error("Unknown process")]
    UnknownProcess,

    #[error("No mailbox")]
    NoMailbox,

    #[error("Dead pubsub")]
    DeadPubSub,

    #[error("Exec error - {0}")]
    ExecError(String),

    #[error("Missing IO Source - {0}")]
    NoIOResource(String),
}
