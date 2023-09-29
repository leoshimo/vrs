use super::kernel;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Failed to message kernel task - {0}")]
    FailedToMessageKernelTask(#[from] tokio::sync::mpsc::error::SendError<kernel::Message>),

    #[error("Failed to receive response from kernel task - {0}")]
    FailedToReceiveResponseFromKernelTask(#[from] tokio::sync::oneshot::error::RecvError),
}
