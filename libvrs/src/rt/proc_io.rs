//! Process IO
use super::kernel::WeakKernelHandle;
use super::mailbox::{MailboxHandle, Message};
use super::pubsub::PubSubHandle;
use super::registry::{Registration, Registry};
use super::ProcessId;
use lyric::{Form, KeywordId};
use std::time::Duration;
use tokio::process::Command;
use tokio::time;
use tracing::{debug, error};

use crate::connection::Error as ConnError;

use crate::{Connection, ProcessHandle, Program, Response};

use super::program::{Extern, Fiber, Pattern, Val};
use crate::rt::{Error, Result};

/// Handles process IO requests
pub(crate) struct ProcIO {
    pid: ProcessId,
    conn: Option<Connection>,
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
    RecvRequest,
    SendResponse(Val),
    ListProcesses,
    KillProcess(ProcessId),
    SendMessage(ProcessId, Val),
    ListMessages,
    Exec(String, Vec<String>),
    Recv(Option<Pattern>),
    Sleep(Duration),
    Spawn(Program),
    RegisterAsService(Registration),
    ListServices,
    QueryService(KeywordId, ServiceQuery),

    Subscribe(KeywordId),
    Publish(KeywordId, Val),
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
            conn: None,
            pending: None,
            kernel: None,
            registry: None,
            pubsub: None,
            mailbox: None,
            self_handle: None,
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
            IOCmd::RecvRequest => self.recv_request().await,
            IOCmd::SendResponse(v) => self.send_response(v).await,
            IOCmd::ListProcesses => self.list_processes().await,
            IOCmd::KillProcess(pid) => self.kill_process(pid).await,
            IOCmd::SendMessage(dst, val) => self.send_message(dst, val).await,
            IOCmd::ListMessages => self.list_message().await,
            IOCmd::Exec(prog, args) => self.exec(prog, args).await,
            IOCmd::Recv(pat) => self.handle_recv(pat).await,
            IOCmd::Sleep(duration) => self.sleep(duration).await,
            IOCmd::Spawn(prog) => self.spawn_prog(prog).await,
            IOCmd::RegisterAsService(reg) => self.register_self(reg).await,
            IOCmd::ListServices => self.list_services().await,
            IOCmd::QueryService(svc, info) => self.query_service(svc, info).await,
            IOCmd::Subscribe(topic) => self.subscribe(topic).await,
            IOCmd::Publish(topic, val) => self.publish(topic, val).await,
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
            .map(|pid| Val::Extern(Extern::ProcessId(pid)))
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
        Ok(Val::keyword("ok"))
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
        let mut cmd = Command::new(prog.clone())
            .args(args.clone())
            .spawn()
            .map_err(|e| Error::IOError(format!("{}", e)))?;
        let exit_status = cmd
            .wait()
            .await
            .map_err(|e| Error::IOError(format!("{}", e)))?;

        if exit_status.success() {
            debug!("exec {:?} {:?} - {:?}", prog, args, exit_status);
            Ok(Val::keyword("ok"))
        } else {
            error!("exec {:?} {:?} - {:?}", prog, args, exit_status);
            Err(Error::ExecError(format!(
                "Failed to execute - {}",
                exit_status
            )))
        }
    }

    /// Sleep process
    async fn sleep(&self, duration: Duration) -> Result<Val> {
        debug!("sleep {:?}", &duration);
        time::sleep(duration).await;
        Ok(Val::keyword("ok"))
    }

    /// Spawn given process
    async fn spawn_prog(&self, prog: Program) -> Result<Val> {
        debug!("spawn_prog {:?}", &prog);
        let kernel = self
            .kernel
            .as_ref()
            .and_then(|k| k.upgrade())
            .ok_or(Error::NoKernel)?;
        let hdl = kernel.spawn_prog(prog).await?;
        Ok(Val::Extern(Extern::ProcessId(hdl.id())))
    }

    /// Register itself as a process
    async fn register_self(&self, reg: Registration) -> Result<Val> {
        let hdl = self.self_handle.as_ref().expect("Dangling ProcIO");

        self.registry
            .as_ref()
            .ok_or(Error::NoIOResource("No registry for process".to_string()))?
            .register(reg, hdl.clone())
            .await?;

        Ok(Val::keyword("ok"))
    }

    /// Retrieve available services as
    async fn list_services(&self) -> Result<Val> {
        let entries = self
            .registry
            .as_ref()
            .ok_or(Error::NoIOResource("No registry for process".to_string()))?
            .all()
            .await?;
        let entry_values: Vec<_> = entries.into_iter().map(Val::from).collect();
        Ok(Val::List(entry_values))
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

    async fn recv_request(&mut self) -> Result<Val> {
        let conn = self.conn.as_mut().ok_or(Error::IOFailed)?;
        if self.pending.is_some() {
            return Err(Error::IOFailed); // HACK: only one pending at a time. Needs Client-equivalent on Runtime-side
        }

        let req = conn
            .recv_req()
            .await
            .ok_or(Error::ConnectionClosed)?
            .map_err(|e| Error::IOError(format!("{}", e)))?;
        self.pending = Some(req.id);
        Ok(req.contents.into())
    }

    async fn send_response(&mut self, v: Val) -> Result<Val> {
        let conn = self.conn.as_mut().ok_or(Error::IOFailed)?;
        let pending = self.pending.take().ok_or(Error::IOFailed)?;
        let contents: lyric::Result<Form> = v.try_into();
        conn.send_resp(Response {
            req_id: pending,
            contents: contents.map_err(ConnError::EvaluationError),
        })
        .await
        .map_err(|e| Error::IOError(format!("{}", e)))?;
        Ok(Val::keyword("ok"))
    }

    async fn subscribe(&self, topic: KeywordId) -> Result<Val> {
        let pubsub = self
            .pubsub
            .as_ref()
            .ok_or(Error::NoIOResource("no pubsub for process".to_string()))?;
        let mb = self.mailbox.as_ref().ok_or(Error::NoMailbox)?.clone();
        let mut sub = pubsub.subscribe(&topic).await?;

        // TODO: Should process keep track of active subscriptions via some task handle?
        tokio::spawn(async move {
            while let Some(ev) = sub.recv().await {
                let msg = Message {
                    contents: Val::List(vec![
                        Val::keyword("topic_updated"),
                        Val::Keyword(topic.clone()),
                        ev,
                    ]),
                };
                if let Err(e) = mb.push(msg).await {
                    error!("Error while pushing subscription event to mailbox - {e}");
                }
            }
        });

        Ok(Val::keyword("ok"))
    }

    // TODO: Implement unsubscribe? ls-subs?

    async fn publish(&self, topic: KeywordId, val: Val) -> Result<Val> {
        let pubsub = self
            .pubsub
            .as_ref()
            .ok_or(Error::NoIOResource("no pubsub for process".to_string()))?;

        pubsub
            .publish(&topic, val)
            .await
            .map_err(|e| Error::ProcessIOError(format!("Failed to publish on pubsub - {e}")))?;

        Ok(Val::keyword("ok"))
    }
}
