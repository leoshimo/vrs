#![allow(dead_code)]
//! Controlling Terminal for Processes

use std::collections::VecDeque;

use crate::connection::Message;
use crate::rt::{Error, Result};
use crate::{Connection, Request};
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, error};

/// Controlling terminal
#[derive(Debug)]
pub struct Term {
    rx: mpsc::Receiver<Cmd>,
    conn: Connection,
    req_queue: VecDeque<Request>,
    read_req_queue: VecDeque<oneshot::Sender<Request>>,
}

/// Handle to [Term]
#[derive(Debug, Clone)]
pub struct TermHandle {
    tx: mpsc::Sender<Cmd>,
}

#[derive(Debug)]
enum Cmd {
    ReadRequest(oneshot::Sender<Request>),
}

impl Term {
    /// Create a new terminal connection
    pub(crate) fn spawn(conn: Connection) -> TermHandle {
        let (tx, rx) = mpsc::channel(32);
        let t = Term {
            rx,
            conn,
            req_queue: Default::default(),
            read_req_queue: Default::default(),
        };
        tokio::spawn(async move {
            if let Err(e) = t.run().await {
                error!("term error - {e}");
            }
        });
        TermHandle { tx }
    }

    /// Run the term task
    async fn run(mut self) -> Result<()> {
        loop {
            tokio::select! {
                req = Term::read_req(&mut self.conn) => {
                    let req = req?;
                    if let Some(tx) = self.read_req_queue.pop_front() {
                        let _ = tx.send(req);
                    } else {
                        self.req_queue.push_back(req);
                    }
                }
                cmd = self.rx.recv() => {
                    let cmd = match cmd {
                        Some(cmd) => cmd,
                        None => break,
                    };
                    match cmd {
                        Cmd::ReadRequest(req_tx) => {
                            match self.req_queue.pop_front() {
                                Some(req) => {
                                    let _ = req_tx.send(req);
                                }
                                None => {
                                    self.read_req_queue.push_back(req_tx);
                                }
                            }
                        }
                    }
                }
            }

            debug!("{:?}", self);
        }
        Ok(())
    }

    /// Async task to poll for new messages
    async fn read_req(conn: &mut Connection) -> Result<Request> {
        let msg = conn
            .recv()
            .await
            .ok_or(Error::ConnectionClosed)?
            .map_err(|e| Error::IOError(format!("{e}")))?;
        let req = match msg {
            Message::Request(req) => req,
            Message::Response(_) => {
                return Err(Error::IOError(format!("Unexpected message: {:?}", msg)));
            }
        };
        Ok(req)
    }
}

impl TermHandle {
    /// Read request from terminal
    pub(crate) async fn read_request(&self) -> Result<Request> {
        let (req_tx, req_rx) = oneshot::channel();
        self.tx
            .send(Cmd::ReadRequest(req_tx))
            .await
            .map_err(|e| Error::NoMessageReceiver(format!("read_request failed - {e}")))?;
        Ok(req_rx.await?)
    }
}

impl std::cmp::PartialEq for TermHandle {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(&self.tx, &other.tx)
    }
}

#[cfg(test)]
mod tests {
    use crate::rt::program::Form;

    use super::*;
    use std::time::Duration;
    use tokio::{task::yield_now, time::timeout};

    #[tokio::test]
    async fn read_req_no_request() {
        let (rt, _client) = Connection::pair().unwrap();
        let t = Term::spawn(rt);

        timeout(Duration::from_secs(0), t.read_request())
            .await
            .expect_err("Expect timeout");
    }

    // TODO: How to test interleaving of oneshot channel request / response after main terminal loop but before oneshot receiver recv?
    #[tokio::test]
    async fn read_request_sequencing() {
        // Send client.send_req before Cmd::ReadRequest
        {
            let (rt, mut client) = Connection::pair().unwrap();
            let t = Term::spawn(rt);

            let (req_tx, req_rx) = oneshot::channel();

            client
                .send_req(Request {
                    id: 10,
                    contents: Form::string("Hello"),
                })
                .await
                .unwrap();

            yield_now().await;
            t.tx.send(Cmd::ReadRequest(req_tx)).await.unwrap();

            let req = req_rx.await.expect("Should receive req");

            assert_eq!(
                req,
                Request {
                    id: 10,
                    contents: Form::string("Hello")
                }
            );
        }

        // Send before Cmd::ReadRequest client.send_req
        {
            let (rt, mut client) = Connection::pair().unwrap();
            let t = Term::spawn(rt);

            let (req_tx, req_rx) = oneshot::channel();

            t.tx.send(Cmd::ReadRequest(req_tx)).await.unwrap();
            client
                .send_req(Request {
                    id: 10,
                    contents: Form::string("Hello"),
                })
                .await
                .unwrap();

            let req = req_rx.await.expect("Should receive req");

            assert_eq!(
                req,
                Request {
                    id: 10,
                    contents: Form::string("Hello")
                }
            );
        }
    }

    #[tokio::test]
    async fn read_request_in_sequence() {
        let (rt, mut client) = Connection::pair().unwrap();

        let t = Term::spawn(rt);

        tokio::spawn(async move {
            for i in 0..5 {
                client
                    .send_req(Request {
                        id: i,
                        contents: Form::Int(i.try_into().unwrap()),
                    })
                    .await
                    .unwrap();
            }
            loop {
                yield_now().await; // don't drop client
            }
        });

        let term_task = tokio::spawn(async move {
            let mut reqs = vec![];
            for _ in 0..5 {
                let req = t.read_request().await.unwrap();
                reqs.push(req);
            }
            reqs
        });

        let reqs = term_task.await.unwrap();
        let expected = (0..5)
            .map(|i| Request {
                id: i,
                contents: Form::Int(i.try_into().unwrap()),
            })
            .collect::<Vec<_>>();
        assert_eq!(reqs, expected, "Requests are returned in order");
    }

    // TODO: Test that read_request fails if connection is dropped
    // TODO: Test that process is killed after connection is disconnected (?)
}
