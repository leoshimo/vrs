//! Process IO
use lyric::Form;

use crate::connection::Error as ConnError;

use crate::{Connection, Response};

use super::proc::Val;
use crate::rt::{Error, Result};

/// Handles process IO requests
pub(crate) struct ProcIO {
    conn: Option<Connection>,
    pending: Option<u32>,
}

/// Set of IO command ProcIO can handle
#[derive(Debug, Clone, PartialEq)]
pub enum IOCmd {
    RecvRequest,
    SendRequest(Val),
}

impl ProcIO {
    /// Create new IO sources for process
    pub(crate) fn new() -> Self {
        Self {
            conn: None,
            pending: None,
        }
    }

    /// Set connection on IO
    pub(crate) fn conn(&mut self, conn: Connection) -> &mut Self {
        self.conn = Some(conn);
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
        }
    }
}
