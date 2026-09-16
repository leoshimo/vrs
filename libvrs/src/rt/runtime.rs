//! Runtime
use super::kernel::{self, KernelHandle};
use super::peer::{PeerHandle, PeerManager};
use super::pubsub::PubSub;
use super::registry::Registry;
use crate::rt::{ProcessHandle, Result};
use crate::{Connection, Program};

pub const DEFAULT_NODE_PORT: u16 = if cfg!(debug_assertions) { 8774 } else { 8773 };

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
        let pubsub = PubSub::spawn();
        let (peers, commands) = PeerHandle::channel();
        let kernel_task = kernel::start(
            node_name.clone(),
            registry.clone(),
            pubsub.clone(),
            Some(peers.clone()),
        );
        PeerManager::start(
            node_name,
            registry,
            pubsub,
            kernel_task.downgrade(),
            commands,
        );
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Client, Form};
    use std::time::Duration;
    use tokio::time::{sleep, timeout};

    async fn client(runtime: &Runtime) -> Client {
        let (local, remote) = Connection::pair().unwrap();
        runtime.handle_conn(remote).await.unwrap();
        Client::new(local)
    }

    #[tokio::test]
    async fn publications_cross_node_links_once_in_both_directions() {
        let alpha_runtime = Runtime::new("alpha");
        let beta_runtime = Runtime::new("beta");
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        drop(listener);
        beta_runtime.listen_for_nodes(port).await.unwrap();
        let alpha = client(&alpha_runtime).await;
        let beta = client(&beta_runtime).await;
        alpha
            .request(
                Form::from_expr(&format!("(configure :nodes '(\"tcp://127.0.0.1:{port}\"))"))
                    .unwrap(),
            )
            .await
            .unwrap()
            .contents
            .unwrap();
        timeout(Duration::from_secs(5), async {
            loop {
                let response = alpha
                    .request(Form::from_expr("(remote! \"beta\" (node_name))").unwrap())
                    .await
                    .unwrap();
                if response.contents == Ok(Form::string("beta")) {
                    break;
                }
                sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("peer link must be ready before publishing");

        let mut alpha_updates = alpha.subscribe("count".into()).await.unwrap();
        let mut beta_updates = beta.subscribe("count".into()).await.unwrap();
        // An evaluation on each connection follows its SubscriptionStart.
        for subscriber in [&alpha, &beta] {
            subscriber
                .request(Form::keyword("ready"))
                .await
                .unwrap()
                .contents
                .unwrap();
        }
        for (publisher, origin) in [(&alpha, "alpha"), (&beta, "beta")] {
            let payload = format!("(:value 42 :origin {origin})");
            publisher
                .request(Form::from_expr(&format!("(publish :count '{payload})")).unwrap())
                .await
                .unwrap()
                .contents
                .unwrap();
            for (node, subscription) in [("alpha", &mut alpha_updates), ("beta", &mut beta_updates)]
            {
                let value = timeout(Duration::from_secs(1), subscription.recv())
                    .await
                    .unwrap_or_else(|_| panic!("publication from {origin} never reached {node}"))
                    .unwrap();
                assert_eq!(value, Form::from_expr(&payload).unwrap());
            }
        }
        for subscription in [&mut alpha_updates, &mut beta_updates] {
            assert!(
                timeout(Duration::from_millis(100), subscription.recv())
                    .await
                    .is_err(),
                "publication was echoed or duplicated"
            );
        }
    }
}
