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
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::connection::tests::conn_fixture;
    use crate::runtime::process;
    use crate::Client;
    use lemma::{parse as p, Form};
    use tracing_test::traced_test;

    #[tokio::test]
    #[traced_test]
    async fn client_connection_request_response() {
        let (local, remote) = conn_fixture();
        let proc = process::spawn_with_sub(Some(Subscription::ClientConnection(local)));
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
}
