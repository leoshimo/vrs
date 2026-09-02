//! Runtime
use super::kernel::{self, KernelHandle};
use crate::rt::{ProcessHandle, Result};
use crate::{Connection, Program};

/// Handle to Runtime's public interface
pub struct Runtime {
    kernel_task: KernelHandle,
}

impl Runtime {
    /// Create a runtime whose processes share one immutable node name.
    pub fn new(node_name: impl Into<String>) -> Self {
        let kernel_task = kernel::start(node_name.into());
        Self { kernel_task }
    }

    /// Notify the runtime of new connection to handle
    pub async fn handle_conn(&self, conn: Connection) -> Result<ProcessHandle> {
        self.kernel_task.spawn_for_conn(conn).await
    }

    /// Spawn a given program
    pub async fn run(&self, prog: Program) -> Result<ProcessHandle> {
        self.kernel_task.spawn_prog(prog).await
    }
}
