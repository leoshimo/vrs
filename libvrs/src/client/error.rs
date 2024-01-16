use super::Event;

/// Errors from interacting with [Client]
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Failed to send on mpsc - {0}")]
    FailedToSend(#[from] tokio::sync::mpsc::error::SendError<Event>),

    #[error("Failed to recv on oneshot - {0}")]
    FailedToRecv(#[from] tokio::sync::oneshot::error::RecvError),

    #[error("Failed to recv on broadcast - {0}")]
    BroadcastFailedToRecv(#[from] tokio::sync::broadcast::error::RecvError),

    #[error("{0}")]
    IO(#[from] std::io::Error),

    #[error("Internal inconsistency - {0}")]
    Internal(String),

    #[error("Connection disconnected")]
    Disconnected,
}
