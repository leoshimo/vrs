//! Process IO
use super::kernel::WeakKernelHandle;
use lyric::Form;

use crate::connection::Error as ConnError;

use crate::{Connection, Response};

use super::proc::Val;
use crate::rt::{Error, Result};

/// Handles process IO requests
pub(crate) struct ProcIO {
    conn: Option<Connection>,
    pending: Option<u32>,
    kernel: Option<WeakKernelHandle>,
}

/// Set of IO command ProcIO can handle
#[derive(Debug, Clone, PartialEq)]
pub enum IOCmd {
    RecvRequest,
    SendRequest(Val),
    ListProcesses,
}

impl ProcIO {
    /// Create new IO sources for process
    pub(crate) fn new() -> Self {
        Self {
            conn: None,
            pending: None,
            kernel: None,
        }
    }

    /// Set connection on IO
    pub(crate) fn conn(&mut self, conn: Connection) -> &mut Self {
        self.conn = Some(conn);
        self
    }

    /// Set kernel handle
    pub(crate) fn kernel(&mut self, kernel: WeakKernelHandle) -> &mut Self {
        self.kernel = Some(kernel);
        self
    }

    /// Poll for IO event
    pub(crate) async fn dispatch_io(&mut self, cmd: IOCmd) -> Result<Val> {
        match cmd {
            IOCmd::RecvRequest => {
                let conn = self.conn.as_mut().ok_or(Error::IOFailed)?;
                if self.pending.is_some() {
                    return Err(Error::IOFailed); // HACK: only one pending at a time
                }

                let req = conn.recv_req().await.ok_or(Error::ConnectionClosed)??;
                self.pending = Some(req.req_id);
                Ok(req.contents.into())
            }
            IOCmd::SendRequest(v) => {
                let conn = self.conn.as_mut().ok_or(Error::IOFailed)?;
                let pending = self.pending.take().ok_or(Error::IOFailed)?;
                let contents: lyric::Result<Form> = v.try_into();
                conn.send_resp(Response {
                    req_id: pending,
                    contents: contents.map_err(ConnError::EvaluationError),
                })
                .await?;
                Ok(Val::symbol("ok"))
            }
            IOCmd::ListProcesses => {
                let kernel = self
                    .kernel
                    .as_ref()
                    .and_then(|k| k.upgrade())
                    .ok_or(Error::NoKernel)?;
                let procs = kernel
                    .procs()
                    .await?
                    .into_iter()
                    .map(|pid| Val::Int(pid as i32))
                    .collect::<Vec<_>>();
                Ok(Val::List(procs))
            }
        }
    }
}
