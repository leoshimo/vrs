//! Runtime
use super::kernel::{self, KernelHandle};
use super::peer::{PeerHandle, PeerManager};
use super::registry::Registry;
use crate::rt::{ProcessHandle, Result};
use crate::{Connection, Program};

pub const DEFAULT_NODE_PORT: u16 = 8773;

/// Handle to Runtime's public interface
pub struct Runtime {
    kernel_task: KernelHandle,
    peers: PeerHandle,
}

impl Runtime {
    /// Create a runtime whose processes share one immutable node name.
    pub fn new(node_name: impl Into<String>) -> Self {
        let node_name = node_name.into();
        let registry = Registry::spawn_named(node_name.clone());
        let (peers, commands) = PeerHandle::channel();
        let kernel_task = kernel::start(node_name.clone(), registry.clone(), Some(peers.clone()));
        PeerManager::start(node_name, registry, kernel_task.downgrade(), commands);
        Self { kernel_task, peers }
    }

    /// Listen for node links on localhost. This is separate from construction
    /// so embedded and local-only runtimes do not open a network port.
    pub async fn listen_for_nodes(&self, port: u16) -> Result<()> {
        self.peers.listen(port).await
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
