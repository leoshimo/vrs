//! Process IO
use super::kernel::WeakKernelHandle;
use super::mailbox::MailboxHandle;
use super::ProcessId;
use lyric::Form;
use tokio::process::Command;
use tracing::{debug, error};

use crate::connection::Error as ConnError;

use crate::{Connection, Response};

use super::proc::{Pattern, Val};
use crate::rt::{Error, Result};

/// Handles process IO requests
pub(crate) struct ProcIO {
    pid: ProcessId,
    conn: Option<Connection>,
    pending: Option<u32>,
    mailbox: Option<MailboxHandle>,
    kernel: Option<WeakKernelHandle>,
}

/// Set of IO command ProcIO can handle
#[derive(Debug, Clone, PartialEq)]
pub enum IOCmd {
    RecvRequest,
    SendRequest(Val),
    ListProcesses,
    KillProcess(ProcessId),
    SendMessage(ProcessId, Val),
    ListMessages,
    Exec(String, Vec<String>),
    Recv(Option<Pattern>),
}

impl ProcIO {
    /// Create new IO sources for given process
    pub(crate) fn new(pid: ProcessId) -> Self {
        Self {
            pid,
            conn: None,
            pending: None,
            kernel: None,
            mailbox: None,
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

    pub(crate) fn mailbox(&mut self, mailbox: MailboxHandle) -> &mut Self {
        self.mailbox = Some(mailbox);
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
            IOCmd::ListProcesses => self.list_processes().await,
            IOCmd::KillProcess(pid) => self.kill_process(pid).await,
            IOCmd::SendMessage(dst, val) => self.send_message(dst, val).await,
            IOCmd::ListMessages => self.list_message().await,
            IOCmd::Exec(prog, args) => self.exec(prog, args).await,
            IOCmd::Recv(pat) => self.handle_recv(pat).await,
        }
    }

    /// List Processes
    async fn list_processes(&self) -> Result<Val> {
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

    /// Kill process
    async fn kill_process(&self, pid: ProcessId) -> Result<Val> {
        let kernel = self
            .kernel
            .as_ref()
            .and_then(|k| k.upgrade())
            .ok_or(Error::NoKernel)?;
        kernel.kill_proc(pid).await?;
        Ok(Val::symbol("ok"))
    }

    /// Send message to another process
    async fn send_message(&self, dst: ProcessId, msg: Val) -> Result<Val> {
        let kernel = self
            .kernel
            .as_ref()
            .and_then(|k| k.upgrade())
            .ok_or(Error::NoKernel)?;
        kernel.send_message(self.pid, dst, msg.clone()).await?;
        Ok(msg)
    }

    /// Handle recv command
    async fn handle_recv(&self, pat: Option<Pattern>) -> Result<Val> {
        let mailbox = self.mailbox.as_ref().ok_or(Error::NoMailbox)?;
        let msg = mailbox.poll(pat).await?;
        Ok(msg.contents)
    }

    /// List messages in mailbox
    async fn list_message(&self) -> Result<Val> {
        let mailbox = self.mailbox.as_ref().ok_or(Error::NoMailbox)?;

        let msgs = mailbox.all().await?;
        let msg_vals = msgs.into_iter().map(|m| m.contents).collect();

        Ok(Val::List(msg_vals))
    }

    /// Execute specified program
    async fn exec(&self, prog: String, args: Vec<String>) -> Result<Val> {
        debug!("exec {:?} {:?}", &prog, &args);
        let mut cmd = Command::new(prog.clone()).args(args.clone()).spawn()?;
        let exit_status = cmd.wait().await?;

        if exit_status.success() {
            debug!("exec {:?} {:?} - {:?}", prog, args, exit_status);
            Ok(Val::symbol("ok"))
        } else {
            error!("exec {:?} {:?} - {:?}", prog, args, exit_status);
            Err(Error::ExecError(format!(
                "Failed to execute - {}",
                exit_status
            )))
        }
    }
}
