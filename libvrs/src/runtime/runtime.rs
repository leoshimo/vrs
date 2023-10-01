//! Runtime
use super::kernel::{self, KernelHandle};
use super::v2::{ProcessId, Result};
use crate::Connection;

/// Handle to Runtime's public interface
pub struct Runtime {
    kernel_task: KernelHandle,
}

impl Runtime {
    /// Create new runtime instance
    pub fn new() -> Self {
        let kernel_task = kernel::start();
        Self { kernel_task }
    }

    /// Notify the runtime of new connection to handle
    pub async fn handle_conn(&self, conn: Connection) -> Result<ProcessId> {
        self.kernel_task.spawn_proc(Some(conn)).await
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Runtime::new()
    }
}
