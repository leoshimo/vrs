//! Remote evaluations and client sessions carried by an existing node link.
//! Each reply carries an atomic, versioned registry checkpoint. The receiver
//! applies it before releasing the reply, including replies from forked services.
use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot};
use tokio::task::AbortHandle;

use super::kernel::WeakKernelHandle;
use super::peer::{PeerMessage, WireVal};
use super::registry::{Registry, RemoteChange, Snapshot};
use crate::connection::Message;
use crate::{Connection, Error, ProcessHandle, ProcessResult, Program, Result, Val};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) enum RemoteMessage {
    Eval {
        id: u64,
        code: WireVal,
    },
    Evaluated {
        id: u64,
        result: std::result::Result<WireVal, lyric::Error>,
        snapshot: Snapshot,
    },
    Cancel {
        id: u64,
    },
    Open {
        id: u64,
    },
    Client {
        id: u64,
        message: Message,
    },
    Server {
        id: u64,
        message: Message,
        snapshot: Option<Snapshot>,
    },
    Close {
        id: u64,
        server: bool,
    },
}

type SessionKey = (u64, bool, u64);

#[derive(Default)]
pub(super) struct RemoteSessions {
    pending: HashMap<(u64, u64), oneshot::Sender<Result<Val>>>,
    evaluations: HashMap<(u64, u64), AbortHandle>,
    sessions: HashMap<SessionKey, (mpsc::Sender<Message>, AbortHandle)>,
}

/// Dropping an evaluation future cancels its evaluator, but not spawned services.
pub(super) struct EvaluatorGuard(pub ProcessHandle);
impl Drop for EvaluatorGuard {
    fn drop(&mut self) {
        let process = self.0.clone();
        tokio::spawn(async move {
            process.kill().await;
        });
    }
}

pub(super) async fn evaluate(kernel: WeakKernelHandle, code: Val) -> Result<Val> {
    let kernel = kernel.upgrade().ok_or(Error::NoKernel)?;
    let process = kernel.spawn_prog(Program::from_val(code)?).await?;
    let _owner = EvaluatorGuard(process.clone());
    match process.join().await?.status? {
        ProcessResult::Done(value) => Ok(value),
        ProcessResult::Cancelled => Err(Error::EvaluationError(lyric::Error::Runtime(
            "remote evaluation cancelled".into(),
        ))),
    }
}

impl RemoteSessions {
    pub(super) async fn start_eval(
        &mut self,
        link: u64,
        tx: &mpsc::Sender<PeerMessage>,
        id: u64,
        code: WireVal,
        reply: oneshot::Sender<Result<Val>>,
    ) {
        self.pending.insert((link, id), reply);
        if tx
            .send(PeerMessage::Remote(RemoteMessage::Eval { id, code }))
            .await
            .is_err()
        {
            self.pending.remove(&(link, id));
        }
    }

    pub(super) async fn cancel_eval(&mut self, link: u64, tx: &mpsc::Sender<PeerMessage>, id: u64) {
        if self.pending.remove(&(link, id)).is_some() {
            let _ = tx
                .send(PeerMessage::Remote(RemoteMessage::Cancel { id }))
                .await;
        }
    }

    pub(super) async fn open(
        &mut self,
        link: u64,
        tx: mpsc::Sender<PeerMessage>,
        id: u64,
        registry: Registry,
    ) -> Result<Connection> {
        let (client, bridge) = Connection::pair().map_err(|e| Error::IOError(e.to_string()))?;
        tx.send(PeerMessage::Remote(RemoteMessage::Open { id }))
            .await
            .map_err(|_| Error::ConnectionClosed)?;
        self.bridge((link, false, id), bridge, tx, registry);
        Ok(client)
    }

    pub(super) async fn incoming(
        &mut self,
        link: u64,
        node: &str,
        tx: mpsc::Sender<PeerMessage>,
        message: RemoteMessage,
        registry: &Registry,
        kernel: &WeakKernelHandle,
    ) -> Result<()> {
        self.evaluations.retain(|_, task| !task.is_finished());
        match message {
            RemoteMessage::Eval { id, code } => {
                // Do not start an in-flight request twice. The transport itself
                // never retries evaluations after a link failure.
                if self.evaluations.contains_key(&(link, id)) {
                    return Ok(());
                }
                let kernel = kernel.clone();
                let registry = registry.clone();
                let task = tokio::spawn(async move {
                    let result = match code.into_val() {
                        Ok(code) => evaluate(kernel, code).await.and_then(WireVal::from_val),
                        Err(e) => Err(e),
                    }
                    .map_err(|e| match e {
                        Error::EvaluationError(e) => e,
                        e => lyric::Error::Runtime(e.to_string()),
                    });
                    if let Ok(snapshot) = registry.checkpoint().await {
                        let _ = tx
                            .send(PeerMessage::Remote(RemoteMessage::Evaluated {
                                id,
                                result,
                                snapshot,
                            }))
                            .await;
                    }
                });
                self.evaluations.insert((link, id), task.abort_handle());
            }
            RemoteMessage::Evaluated {
                id,
                result,
                snapshot,
            } => {
                registry
                    .apply_remote(node.to_string(), RemoteChange::Snapshot(snapshot))
                    .await?;
                if let Some(reply) = self.pending.remove(&(link, id)) {
                    let _ = reply.send(
                        result
                            .map_err(Error::EvaluationError)
                            .and_then(WireVal::into_val),
                    );
                }
            }
            RemoteMessage::Cancel { id } => {
                if let Some(task) = self.evaluations.remove(&(link, id)) {
                    task.abort();
                }
            }
            RemoteMessage::Open { id } => {
                if self.sessions.contains_key(&(link, true, id)) {
                    return Ok(());
                }
                let (bridge, terminal) =
                    Connection::pair().map_err(|e| Error::IOError(e.to_string()))?;
                kernel
                    .upgrade()
                    .ok_or(Error::NoKernel)?
                    .spawn_for_conn(terminal)
                    .await?;
                self.bridge((link, true, id), bridge, tx, registry.clone());
            }
            RemoteMessage::Client { id, message } => {
                self.deliver((link, true, id), message).await;
            }
            RemoteMessage::Server {
                id,
                message,
                snapshot,
            } => {
                if let Some(snapshot) = snapshot {
                    registry
                        .apply_remote(node.to_string(), RemoteChange::Snapshot(snapshot))
                        .await?;
                }
                self.deliver((link, false, id), message).await;
            }
            RemoteMessage::Close { id, server } => {
                if let Some((_, task)) = self.sessions.remove(&(link, !server, id)) {
                    task.abort();
                }
            }
        }
        Ok(())
    }

    async fn deliver(&mut self, key: SessionKey, message: Message) {
        if let Some((tx, _)) = self.sessions.get(&key) {
            // A slow client must not prevent heartbeats or other node requests.
            if tx.try_send(message).is_err() {
                if let Some((_, task)) = self.sessions.remove(&key) {
                    task.abort();
                }
            }
        }
    }

    fn bridge(
        &mut self,
        key: SessionKey,
        mut connection: Connection,
        peer: mpsc::Sender<PeerMessage>,
        registry: Registry,
    ) {
        let (_, server, id) = key;
        let (tx, mut rx) = mpsc::channel(64);
        let task = tokio::spawn(async move {
            let _close = CloseSession {
                peer: peer.clone(),
                server,
                id,
            };
            loop {
                tokio::select! {
                    incoming = rx.recv() => {
                        let Some(message) = incoming else { break };
                        if connection.send(&message).await.is_err() { break; }
                    }
                    outgoing = connection.recv() => {
                        let Some(Ok(message)) = outgoing else { break };
                        let frame = if server {
                            let snapshot = if matches!(message, Message::Response(_)) {
                                match registry.checkpoint().await { Ok(s) => Some(s), Err(_) => break }
                            } else { None };
                            RemoteMessage::Server { id, message, snapshot }
                        } else { RemoteMessage::Client { id, message } };
                        if peer.send(PeerMessage::Remote(frame)).await.is_err() { break; }
                    }
                }
            }
        });
        self.sessions.insert(key, (tx, task.abort_handle()));
    }

    pub(super) fn disconnect(&mut self, link: u64) {
        self.pending.retain(|(session, _), _| *session != link);
        self.evaluations.retain(|(session, _), task| {
            if *session == link {
                task.abort();
                false
            } else {
                true
            }
        });
        self.sessions.retain(|(session, _, _), (_, task)| {
            if *session == link {
                task.abort();
                false
            } else {
                true
            }
        });
    }
}

impl Drop for RemoteSessions {
    fn drop(&mut self) {
        for task in self.evaluations.values() {
            task.abort();
        }
        for (_, task) in self.sessions.values() {
            task.abort();
        }
    }
}

struct CloseSession {
    peer: mpsc::Sender<PeerMessage>,
    server: bool,
    id: u64,
}
impl Drop for CloseSession {
    fn drop(&mut self) {
        let peer = self.peer.clone();
        let frame = RemoteMessage::Close {
            id: self.id,
            server: self.server,
        };
        tokio::spawn(async move {
            let _ = peer.send(PeerMessage::Remote(frame)).await;
        });
    }
}
