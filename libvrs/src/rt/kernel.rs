//! Runtime Kernel Task
use super::proc::{ProcessExit, ProcessHandle, ProcessSet};
use crate::rt::{proc::Process, Error, ProcessId, Result};
use crate::Connection;
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, info};

/// Starts the kernel task, which manages processes on runtime
pub(crate) fn start() -> KernelHandle {
    let (ev_tx, mut ev_rx) = mpsc::channel(32);

    let handle = KernelHandle { ev_tx };
    let mut kernel = Kernel::new(handle.clone());
    tokio::spawn(async move {
        loop {
            tokio::select! {
                ev = ev_rx.recv() => match ev {
                    Some(ev) => kernel.handle_ev(ev).await?,
                    None => {
                        info!("Kernel handle dropped - terminating");
                        break;
                    }
                },
                Some(Ok(result)) = kernel.procs.join_next() => {
                    kernel.handle_ev(Event::ProcessExit(result)).await?;
                }
            }
        }
        Ok::<(), Error>(())
    });
    handle
}

/// Handle to `Kernel`
#[derive(Clone)]
pub(crate) struct KernelHandle {
    ev_tx: mpsc::Sender<Event>,
}

impl KernelHandle {
    /// Spawn a new process
    pub(crate) async fn spawn_for_conn(&self, conn: Connection) -> Result<ProcessHandle> {
        let (tx, rx) = oneshot::channel();
        self.ev_tx
            .send(Event::SpawnConnProc(conn, tx))
            .await
            .map_err(|_| Error::FailedToMessageKernel("spawn_for_conn failed".to_string()))?;
        rx.await
            .map_err(Error::FailedToReceiveResponseFromKernelTask)
    }
}

/// Messages for [Kernel]
#[derive(Debug)]
pub enum Event {
    SpawnConnProc(Connection, oneshot::Sender<ProcessHandle>),
    ProcessExit(ProcessExit),
}

/// The runtime kernel task
struct Kernel {
    procs: ProcessSet,
    next_proc_id: usize,
}

impl Kernel {
    pub fn new(_handle: KernelHandle) -> Self {
        Self {
            procs: ProcessSet::new(),
            next_proc_id: 0,
        }
    }

    pub async fn handle_ev(&mut self, ev: Event) -> Result<()> {
        debug!("handle_ev - {ev:?}");
        match ev {
            Event::SpawnConnProc(conn, rx) => {
                let p = self.spawn_conn_proc(conn).await?;
                let _ = rx.send(p);
                Ok(())
            }
            Event::ProcessExit(exit) => self.handle_exit(exit),
        }
    }

    /// Spawn a new process for given connection
    async fn spawn_conn_proc(&mut self, conn: Connection) -> Result<ProcessHandle> {
        let id = ProcessId::from(self.next_proc_id);
        debug!("spawn_proc {id:?}");

        self.next_proc_id = self.next_proc_id.wrapping_add(1);
        let p = Process::from_expr(id, "(loop (send_resp (peval (recv_req))))")
            .unwrap()
            .conn(conn)
            .spawn(&mut self.procs)?;

        Ok(p)
    }

    /// Cleanup process that terminated with given result
    fn handle_exit(&mut self, exit: ProcessExit) -> Result<()> {
        debug!("handle_exit - {exit:?}");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{Client, Connection};
    use lyric::{parse as p, Form};

    use super::*;

    #[tokio::test]
    async fn kernel_proc_for_conn() {
        let (local, remote) = Connection::pair().unwrap();
        let mut client = Client::new(remote);

        let k = start();
        let _ = k
            .spawn_for_conn(local)
            .await
            .expect("Kernel should spawn new process");

        let _ = client
            .request(p("(def msg \"Hello world\")").unwrap())
            .await
            .expect("Client should send request");

        // Verify via client
        let resp = client
            .request(p("msg").unwrap())
            .await
            .expect("Client should send request");
        assert_eq!(resp.contents, Ok(Form::string("Hello world")));
    }
}
