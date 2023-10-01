//! Subscriptions for processes
use super::v2::WeakProcessHandle;
use crate::{
    connection::{Connection, Message},
    Response,
};
use tokio::task::JoinHandle;
use tracing::{debug, error};

/// The underlying task backing subscription messaging
type SubscriptionTask = JoinHandle<()>;

/// IDs assigned to subscriptions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SubscriptionId(usize);

impl From<usize> for SubscriptionId {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl std::fmt::Display for SubscriptionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Handle to subscription
pub(crate) struct SubscriptionHandle {
    task: SubscriptionTask,
}

/// Represents a pending subscription
#[derive(Debug)]
pub enum Subscription {
    ClientConnection(Connection),
}

/// Start the messages for given subscription targeting given process
pub(crate) fn start(
    id: SubscriptionId,
    sub: Subscription,
    target: WeakProcessHandle,
) -> SubscriptionHandle {
    debug!("Starting subscription {id} for {sub:?}");

    use Subscription::*;
    let task = match sub {
        ClientConnection(c) => subscription_for_client_connection(c, target),
    };
    SubscriptionHandle {
        task: tokio::spawn(task),
    }
}

impl SubscriptionHandle {
    /// Abort an active subscription
    pub(crate) fn abort(self) {
        self.task.abort()
    }
}

/// Create a new subscription for receiving messages over connection
async fn subscription_for_client_connection(mut conn: Connection, target: WeakProcessHandle) {
    while let Some(msg) = conn.recv().await {
        let req = match msg {
            Ok(Message::Request(req)) => req,
            _ => {
                error!("Subscription received unexpected message - {msg:?}");
                break;
            }
        };

        let target = match target.upgrade() {
            Some(t) => t,
            None => break,
        };

        let resp_contents = match target.call(req.contents).await {
            Ok(r) => r,
            Err(e) => {
                // TODO: Propagate error to client and carry on?
                error!("Encountered error evaluating expression - {e}");
                break;
            }
        };

        let resp = Message::Response(Response {
            req_id: req.req_id,
            contents: resp_contents,
        });

        match conn.send(&resp).await {
            Ok(_) => (),
            Err(e) => {
                error!("Error while sending response - {e}");
                break;
            }
        };
    }

    // When conn terminates, trigger graceful shutdown
    if let Some(target) = target.upgrade() {
        let _ = target.shutdown().await;
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::runtime::process::fixture::spawn_proc_fixture;
    use crate::Client;
    use crate::{connection::tests::conn_fixture, runtime::process::ProcessSet};
    use lemma::{parse as p, Form};
    use tokio::task::yield_now;
    use tracing_test::traced_test;

    // TODO: Move to integration test?
    #[tokio::test]
    #[traced_test]
    async fn client_connection_request_response() {
        let (local, remote) = conn_fixture();
        let mut proc_set = ProcessSet::new();
        let proc = spawn_proc_fixture(&mut proc_set);

        proc.add_subscription(Subscription::ClientConnection(local))
            .await
            .expect("Adding subscription should succeed");
        let mut remote = Client::new(remote);

        // Process has definitions
        proc.call(p("(def count 0)").unwrap())
            .await
            .expect("count should be defined");
        proc.call(p("(def inc (lambda (x) (+ x 1)))").unwrap())
            .await
            .expect("inc should be defined");

        // Client requests are received + processed
        let resp = remote
            .request(p("count").expect("Request should send"))
            .await
            .expect("Response should return");
        assert_eq!(resp.contents, Form::Int(0));
    }

    #[tokio::test]
    #[traced_test]
    async fn client_connection_drop() {
        use std::time::Duration;
        use tokio::time::timeout;

        let (local, remote) = conn_fixture();
        let mut proc_set = ProcessSet::new();
        let proc = spawn_proc_fixture(&mut proc_set);

        proc.add_subscription(Subscription::ClientConnection(local))
            .await
            .expect("Adding subscription should succeed");

        // Connection terminated
        drop(remote);

        // Check it eventually shuts downs
        // Necessary, since shutdown message comes from separate task (subscription task) v.s. the is_shutdown message from this task
        timeout(Duration::from_secs(1), async {
            loop {
                if proc.is_shutdown().await.unwrap() {
                    break;
                }
                yield_now().await;
            }
        })
        .await
        .expect("Should shutdown soon");
    }
}
