//! Subscriptions for processes
use crate::{
    connection::{self, Connection, Message},
    rt,
    rt::process::WeakProcessHandle,
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
                continue;
            }
        };

        let target = match target.upgrade() {
            Some(t) => t,
            None => break,
        };

        let resp = match target.call(req.contents).await {
            Ok(contents) => Response {
                req_id: req.req_id,
                contents: Ok(contents),
            },
            Err(rt::Error::EvaluationError(e)) => Response {
                req_id: req.req_id,
                contents: Err(e.into()),
            },
            Err(e) => {
                error!("Encountered error evaluating expression - {e}");
                Response {
                    req_id: req.req_id,
                    contents: Err(connection::Error::UnexpectedError),
                }
            }
        };

        match conn.send(&Message::Response(resp)).await {
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

// TODO: Reenable tests
// #[cfg(test)]
// mod tests {

//     use super::*;
//     use crate::{rt::kernel, Client, Connection};
//     use lemma::{parse as p, Form};
//     use tokio::task::yield_now;
//     use tracing_test::traced_test;

//     #[tokio::test]
//     #[traced_test]
//     async fn client_connection_request_response() {
//         let (local, remote) = Connection::pair().unwrap();
//         let k = kernel::start();
//         let proc = k.spawn_proc(None).await.unwrap();

//         proc.add_subscription(Subscription::ClientConnection(local))
//             .await
//             .expect("Adding subscription should succeed");
//         let mut remote = Client::new(remote);

//         // Process has definitions
//         proc.call(p("(def count 0)").unwrap())
//             .await
//             .expect("count should be defined");
//         proc.call(p("(def inc (lambda (x) (+ x 1)))").unwrap())
//             .await
//             .expect("inc should be defined");

//         // Client requests are received + processed
//         let resp = remote
//             .request(p("count").expect("Request should send"))
//             .await
//             .expect("Response should return");
//         assert_eq!(resp.contents, Ok(Form::Int(0)));
//     }

//     #[tokio::test]
//     #[traced_test]
//     async fn client_connection_drop() {
//         use std::time::Duration;
//         use tokio::time::timeout;

//         let (local, remote) = Connection::pair().unwrap();
//         let k = kernel::start();
//         let proc = k.spawn_proc(None).await.unwrap();

//         proc.add_subscription(Subscription::ClientConnection(local))
//             .await
//             .expect("Adding subscription should succeed");

//         // Connection terminated
//         drop(remote);

//         // Check it eventually shuts downs
//         // Necessary, since shutdown message comes from separate task (subscription task) v.s. the is_shutdown message from this task
//         timeout(Duration::from_secs(1), async {
//             loop {
//                 if proc.is_shutdown().await.unwrap() {
//                     break;
//                 }
//                 yield_now().await;
//             }
//         })
//         .await
//         .expect("Should shutdown soon");
//     }
// }
