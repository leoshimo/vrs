#![allow(dead_code)] // TODO: Migrate to NativeAsyncFn bindings
//! Process IO
use super::kernel::WeakKernelHandle;
use super::mailbox::MailboxHandle;
use super::pubsub::PubSubHandle;
use super::registry::Registry;
use super::ProcessId;

use crate::ProcessHandle;

use super::program::{Extern, Fiber, KeywordId, Val};
use crate::rt::{Error, Result};

/// Handles process IO requests
#[derive(Debug, Clone)]
pub(crate) struct ProcIO {
    pid: ProcessId,
    // conn: Option<Connection>,
    pending: Option<u32>,
    mailbox: Option<MailboxHandle>,
    registry: Option<Registry>,
    pubsub: Option<PubSubHandle>,
    kernel: Option<WeakKernelHandle>,
    self_handle: Option<ProcessHandle>,
}

/// Set of IO command ProcIO can handle
#[derive(Debug, Clone, PartialEq)]
pub enum IOCmd {
    // RecvRequest,
    // SendResponse(Val),
}

/// Options for QueryService
#[derive(Debug, Clone, PartialEq)]
pub enum ServiceQuery {
    Pid,
    Interface,
}

impl ProcIO {
    /// Create new IO sources for given process
    pub(crate) fn new(pid: ProcessId) -> Self {
        Self {
            pid,
            pending: None,
            kernel: None,
            registry: None,
            pubsub: None,
            mailbox: None,
            self_handle: None,
        }
    }

    /// Set kernel handle
    pub(crate) fn kernel(&mut self, kernel: WeakKernelHandle) -> &mut Self {
        self.kernel = Some(kernel);
        self
    }

    pub(crate) fn mailbox(&mut self, mailbox: MailboxHandle) -> &mut Self {
        self.mailbox = Some(mailbox);
        self
    }

    pub(crate) fn registry(&mut self, registry: Registry) -> &mut Self {
        self.registry = Some(registry);
        self
    }

    pub(crate) fn pubsub(&mut self, pubsub: PubSubHandle) -> &mut Self {
        self.pubsub = Some(pubsub);
        self
    }

    pub(crate) fn handle(&mut self, handle: ProcessHandle) -> &mut Self {
        self.self_handle = Some(handle);
        self
    }

    /// Poll for IO event
    pub(crate) async fn dispatch_io(&mut self, _fiber: &mut Fiber, cmd: IOCmd) -> Result<Val> {
        match cmd {
            // IOCmd::RecvRequest => self.recv_request().await,
            // IOCmd::SendResponse(v) => self.send_response(v).await,
        }
    }

    async fn query_service(&self, keyword: KeywordId, query: ServiceQuery) -> Result<Val> {
        let entry = self
            .registry
            .as_ref()
            .ok_or(Error::NoIOResource("No registry for process".to_string()))?
            .lookup(keyword)
            .await?
            .ok_or(Error::RegistryError("No service found".to_string()))?;

        match query {
            ServiceQuery::Pid => Ok(Val::Extern(Extern::ProcessId(entry.pid()))),
            ServiceQuery::Interface => Ok(Val::List(entry.interface().to_vec())),
        }
    }

    // async fn recv_request(&mut self) -> Result<Val> {
    //     let conn = self.conn.as_mut().ok_or(Error::IOFailed)?;
    //     if self.pending.is_some() {
    //         return Err(Error::IOFailed); // HACK: only one pending at a time. Needs Client-equivalent on Runtime-side
    //     }

    //     let req = conn
    //         .recv_req()
    //         .await
    //         .ok_or(Error::ConnectionClosed)?
    //         .map_err(|e| Error::IOError(format!("{}", e)))?;
    //     self.pending = Some(req.id);
    //     Ok(req.contents.into())
    // }

    // async fn send_response(&mut self, v: Val) -> Result<Val> {
    //     let conn = self.conn.as_mut().ok_or(Error::IOFailed)?;
    //     let pending = self.pending.take().ok_or(Error::IOFailed)?;
    //     let contents: lyric::Result<Form> = v.try_into();
    //     conn.send_resp(Response {
    //         req_id: pending,
    //         contents: contents.map_err(ConnError::EvaluationError),
    //     })
    //     .await
    //     .map_err(|e| Error::IOError(format!("{}", e)))?;
    //     Ok(Val::keyword("ok"))
    // }
}
