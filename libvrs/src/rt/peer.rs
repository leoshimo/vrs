//! Persistent node links and the wire format used to synchronize services.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::process::Stdio;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tracing::{debug, error, info, warn};

use super::kernel::WeakKernelHandle;
use super::program::{Extern, Val};
use super::registry::{Registry, RegistryEvent, ServiceDescription};
use super::runtime::DEFAULT_NODE_PORT;
use crate::{Error, Result};

use super::ProcessId;

static NEXT_SESSION_ID: AtomicU64 = AtomicU64::new(1);
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Clone)]
pub(crate) struct PeerHandle {
    tx: mpsc::Sender<ManagerCmd>,
}

#[derive(Debug)]
pub(crate) struct PeerManager;

#[derive(Debug)]
pub(crate) enum ManagerCmd {
    Listen {
        port: u16,
        response: oneshot::Sender<Result<()>>,
    },
    Configure(Vec<String>),
    Route {
        pid: ProcessId,
        contents: Val,
        response: oneshot::Sender<Result<()>>,
    },
}

#[derive(Debug)]
enum SessionEvent {
    Connected {
        id: u64,
        node: String,
        direction: Direction,
        tx: mpsc::Sender<PeerMessage>,
        shutdown: oneshot::Sender<()>,
    },
    Incoming {
        id: u64,
        node: String,
        message: PeerMessage,
    },
    Disconnected {
        id: u64,
        node: String,
    },
}

#[derive(Debug)]
struct ActiveSession {
    id: u64,
    direction: Direction,
    tx: mpsc::Sender<PeerMessage>,
    shutdown: oneshot::Sender<()>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Direction {
    Incoming,
    Outgoing,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum NodeEndpoint {
    Tcp(String),
    Ssh { host: String, port: u16 },
}

struct NodeLink {
    read: Box<dyn AsyncRead + Unpin + Send>,
    write: Box<dyn AsyncWrite + Unpin + Send>,
    child: Option<Child>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum PeerMessage {
    Hello {
        node: String,
    },
    Heartbeat,
    RegistrySnapshot {
        services: Vec<ServiceDescription>,
    },
    RegistryUp {
        service: ServiceDescription,
    },
    RegistryDown {
        name: lyric::KeywordId,
        pid: ProcessId,
    },
    Deliver {
        pid: ProcessId,
        contents: WireVal,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
enum WireVal {
    Nil,
    Bool(bool),
    Int(i32),
    String(String),
    Symbol(lyric::SymbolId),
    Keyword(lyric::KeywordId),
    List(Vec<WireVal>),
    Error(lyric::Error),
    Ref(lyric::Ref),
    Process(ProcessId),
}

impl PeerHandle {
    pub(crate) fn channel() -> (Self, mpsc::Receiver<ManagerCmd>) {
        let (tx, rx) = mpsc::channel(64);
        (Self { tx }, rx)
    }

    pub(crate) async fn listen(&self, port: u16) -> Result<()> {
        let (response, result) = oneshot::channel();
        self.tx
            .send(ManagerCmd::Listen { port, response })
            .await
            .map_err(|_| Error::NoMessageReceiver("node manager is unavailable".to_string()))?;
        result
            .await
            .map_err(|_| Error::NoMessageReceiver("node manager dropped listener".to_string()))?
    }

    pub(crate) async fn configure(&self, nodes: Vec<String>) -> Result<()> {
        self.tx
            .send(ManagerCmd::Configure(nodes))
            .await
            .map_err(|_| Error::NoMessageReceiver("node manager is unavailable".to_string()))
    }

    pub(crate) async fn route(&self, pid: ProcessId, contents: Val) -> Result<()> {
        let (response, result) = oneshot::channel();
        self.tx
            .send(ManagerCmd::Route {
                pid,
                contents,
                response,
            })
            .await
            .map_err(|_| Error::NoMessageReceiver("node manager is unavailable".to_string()))?;
        result
            .await
            .map_err(|_| Error::NoMessageReceiver("node manager dropped delivery".to_string()))?
    }
}

impl PartialEq for PeerHandle {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(&self.tx, &other.tx)
    }
}

impl PeerManager {
    pub(crate) fn start(
        node_name: String,
        registry: Registry,
        kernel: WeakKernelHandle,
        mut commands: mpsc::Receiver<ManagerCmd>,
    ) {
        let (session_tx, mut session_rx) = mpsc::channel(64);

        tokio::spawn(async move {
            let mut desired = HashSet::new();
            let mut sessions: HashMap<String, ActiveSession> = HashMap::new();
            let mut registry_events = registry.subscribe();
            let mut listening_on = None;

            loop {
                tokio::select! {
                    Some(command) = commands.recv() => match command {
                        ManagerCmd::Listen { port, response } => {
                            if let Some(active_port) = listening_on {
                                let result = if active_port == port {
                                    Ok(())
                                } else {
                                    Err(Error::IOError(format!(
                                        "node listener is already using port {active_port}"
                                    )))
                                };
                                let _ = response.send(result);
                                continue;
                            }

                            match TcpListener::bind(("127.0.0.1", port)).await {
                                Ok(listener) => {
                                    listening_on = Some(port);
                                    let events = session_tx.clone();
                                    let local_node = node_name.clone();
                                    tokio::spawn(async move {
                                        if let Err(e) = accept_sessions(listener, local_node, events).await {
                                            error!("node listener failed: {e}");
                                        }
                                    });
                                    let _ = response.send(Ok(()));
                                }
                                Err(e) => {
                                    let _ = response.send(Err(Error::IOError(format!(
                                        "failed to listen on 127.0.0.1:{port}: {e}"
                                    ))));
                                }
                            }
                        }
                        ManagerCmd::Configure(nodes) => {
                            for node in nodes {
                                let Some(endpoint) = NodeEndpoint::parse(&node) else {
                                    warn!("ignoring invalid node endpoint: {node}");
                                    continue;
                                };
                                if !desired.insert(endpoint.clone()) {
                                    continue;
                                }
                                let events = session_tx.clone();
                                let local_node = node_name.clone();
                                tokio::spawn(connect_loop(endpoint, local_node, events));
                            }
                        }
                        ManagerCmd::Route { pid, contents, response } => {
                            let node = pid.node().to_string();
                            let Some(session) = sessions.get(&node) else {
                                let _ = response.send(Err(Error::NoMessageReceiver(format!(
                                    "node {node} is not connected"
                                ))));
                                continue;
                            };
                            match WireVal::from_val(contents) {
                                Ok(contents) => {
                                    if session.tx.send(PeerMessage::Deliver { pid, contents }).await.is_err() {
                                        let _ = response.send(Err(Error::NoMessageReceiver(format!(
                                            "node {node} disconnected while routing message"
                                        ))));
                                    } else {
                                        let _ = response.send(Ok(()));
                                    }
                                }
                                Err(e) => {
                                    let _ = response.send(Err(e));
                                }
                            }
                        }
                    },
                    event = registry_events.recv() => match event {
                        Ok(RegistryEvent::Up(service)) => {
                            broadcast(&sessions, PeerMessage::RegistryUp { service }).await;
                        }
                        Ok(RegistryEvent::Down { name, pid }) => {
                            broadcast(&sessions, PeerMessage::RegistryDown { name, pid }).await;
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                            if let Ok(services) = registry.local_snapshot().await {
                                broadcast(&sessions, PeerMessage::RegistrySnapshot { services }).await;
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    },
                    Some(event) = session_rx.recv() => match event {
                        SessionEvent::Connected { id, node, direction, tx, shutdown } => {
                            if node == node_name {
                                let _ = shutdown.send(());
                                continue;
                            }
                            if let Some(active) = sessions.get(&node) {
                                let preferred = preferred_direction(&node_name, &node);
                                if direction != preferred || active.direction == preferred {
                                    let _ = shutdown.send(());
                                    continue;
                                }
                                if let Some(active) = sessions.remove(&node) {
                                    let _ = active.shutdown.send(());
                                }
                            }
                            info!("node connected: {node}");
                            sessions.insert(node.clone(), ActiveSession {
                                id,
                                direction,
                                tx: tx.clone(),
                                shutdown,
                            });
                            if let Ok(services) = registry.local_snapshot().await {
                                let _ = tx.send(PeerMessage::RegistrySnapshot { services }).await;
                            }
                        }
                        SessionEvent::Incoming { id, node, message } => {
                            if !sessions.get(&node).is_some_and(|active| active.id == id) {
                                continue;
                            }
                            match message {
                                PeerMessage::RegistrySnapshot { services } => {
                                    let _ = registry.replace_remote(node, services).await;
                                }
                                PeerMessage::RegistryUp { service } if service.pid.node() == node => {
                                    let _ = registry.remote_up(service).await;
                                }
                                PeerMessage::RegistryDown { name, pid } if pid.node() == node => {
                                    let _ = registry.remote_down(node, name, pid).await;
                                }
                                PeerMessage::Deliver { pid, contents } => match contents.into_val() {
                                    Ok(contents) => {
                                        if pid.node() != node_name {
                                            warn!("node {node} tried to deliver to foreign process {pid}");
                                        } else if let Some(kernel) = kernel.upgrade() {
                                            if let Err(e) = kernel.send_message(pid, contents).await {
                                                debug!("remote delivery failed: {e}");
                                            }
                                        }
                                    }
                                    Err(e) => warn!("invalid message from node {node}: {e}"),
                                },
                                PeerMessage::Hello { .. }
                                | PeerMessage::Heartbeat
                                | PeerMessage::RegistryUp { .. }
                                | PeerMessage::RegistryDown { .. } => {}
                            }
                        }
                        SessionEvent::Disconnected { id, node } => {
                            if sessions.get(&node).is_some_and(|active| active.id == id) {
                                info!("node disconnected: {node}");
                                sessions.remove(&node);
                                let _ = registry.remove_node(node).await;
                            }
                        }
                    },
                    else => break,
                }
            }
        });
    }
}

async fn broadcast(sessions: &HashMap<String, ActiveSession>, message: PeerMessage) {
    for session in sessions.values() {
        let _ = session.tx.send(message.clone()).await;
    }
}

fn preferred_direction(local_node: &str, remote_node: &str) -> Direction {
    if local_node < remote_node {
        Direction::Outgoing
    } else {
        Direction::Incoming
    }
}

fn valid_node_name(node: &str) -> bool {
    !node.is_empty()
        && node
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
}

fn valid_ssh_host(host: &str) -> bool {
    !host.starts_with('-')
        && !host.is_empty()
        && host
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_' | '@'))
}

impl NodeEndpoint {
    fn parse(endpoint: &str) -> Option<Self> {
        if let Some(address) = endpoint.strip_prefix("tcp://") {
            return tcp_address(address).map(Self::Tcp);
        }
        if let Some(host) = endpoint.strip_prefix("ssh://") {
            return ssh_target(host).map(|(host, port)| Self::Ssh { host, port });
        }
        None
    }

    fn label(&self) -> String {
        match self {
            Self::Tcp(address) => format!("tcp://{address}"),
            Self::Ssh { host, port } if *port == DEFAULT_NODE_PORT => {
                format!("ssh://{host}")
            }
            Self::Ssh { host, port } => format!("ssh://{host}:{port}"),
        }
    }

    async fn connect(&self) -> std::io::Result<NodeLink> {
        match self {
            Self::Tcp(address) => {
                let (read, write) = TcpStream::connect(address).await?.into_split();
                Ok(NodeLink {
                    read: Box::new(read),
                    write: Box::new(write),
                    child: None,
                })
            }
            Self::Ssh { host, port } => {
                let mut child = ssh_command(host, *port).spawn()?;
                let read = child.stdout.take().ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::BrokenPipe,
                        "ssh did not provide stdout",
                    )
                })?;
                let write = child.stdin.take().ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::BrokenPipe, "ssh did not provide stdin")
                })?;
                Ok(NodeLink {
                    read: Box::new(read),
                    write: Box::new(write),
                    child: Some(child),
                })
            }
        }
    }
}

fn ssh_command(host: &str, port: u16) -> Command {
    let mut command = Command::new("ssh");
    command
        .arg("-T")
        .arg("-o")
        .arg("BatchMode=yes")
        .arg("-W")
        .arg(format!("127.0.0.1:{port}"))
        .arg(host)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .kill_on_drop(true);
    command
}

fn tcp_address(address: &str) -> Option<String> {
    if valid_node_name(address) {
        return Some(format!("{address}:{DEFAULT_NODE_PORT}"));
    }
    let (host, port) = address.rsplit_once(':')?;
    if !valid_node_name(host) || port.parse::<u16>().is_err() {
        return None;
    }
    Some(address.to_string())
}

fn ssh_target(target: &str) -> Option<(String, u16)> {
    if valid_ssh_host(target) {
        return Some((target.to_string(), DEFAULT_NODE_PORT));
    }
    let (host, port) = target.rsplit_once(':')?;
    let port = port.parse::<u16>().ok()?;
    valid_ssh_host(host).then(|| (host.to_string(), port))
}

async fn accept_sessions(
    listener: TcpListener,
    node_name: String,
    events: mpsc::Sender<SessionEvent>,
) -> std::io::Result<()> {
    info!("node listener ready on {}", listener.local_addr()?);
    loop {
        let (stream, _) = listener.accept().await?;
        let (read, write) = stream.into_split();
        tokio::spawn(run_session(
            read,
            write,
            node_name.clone(),
            Direction::Incoming,
            events.clone(),
        ));
    }
}

async fn connect_loop(
    endpoint: NodeEndpoint,
    local_node: String,
    events: mpsc::Sender<SessionEvent>,
) {
    loop {
        let label = endpoint.label();
        let NodeLink {
            read,
            write,
            mut child,
        } = match endpoint.connect().await {
            Ok(link) => link,
            Err(e) => {
                debug!("failed to connect to node endpoint {label}: {e}");
                tokio::time::sleep(Duration::from_secs(2)).await;
                continue;
            }
        };
        run_session(
            read,
            write,
            local_node.clone(),
            Direction::Outgoing,
            events.clone(),
        )
        .await;
        if let Some(child) = child.as_mut() {
            let _ = child.start_kill();
            let _ = child.wait().await;
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}

async fn run_session<R, W>(
    read: R,
    write: W,
    local_node: String,
    direction: Direction,
    events: mpsc::Sender<SessionEvent>,
) where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    run_session_with_heartbeat(
        read,
        write,
        local_node,
        direction,
        events,
        HEARTBEAT_INTERVAL,
        HEARTBEAT_TIMEOUT,
    )
    .await;
}

async fn run_session_with_heartbeat<R, W>(
    read: R,
    mut write: W,
    local_node: String,
    direction: Direction,
    events: mpsc::Sender<SessionEvent>,
    heartbeat_interval: Duration,
    heartbeat_timeout: Duration,
) where
    R: AsyncRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let id = NEXT_SESSION_ID.fetch_add(1, Ordering::Relaxed);
    let hello = PeerMessage::Hello { node: local_node };
    if write_message(&mut write, &hello).await.is_err() {
        return;
    }

    let mut lines = BufReader::new(read).lines();
    let (out_tx, mut out_rx) = mpsc::channel(64);
    let (shutdown_tx, mut shutdown_rx) = oneshot::channel();
    let mut shutdown_tx = Some(shutdown_tx);
    let mut remote_node = None;
    let mut heartbeat = tokio::time::interval(heartbeat_interval);
    heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    heartbeat.tick().await;
    let silence = tokio::time::sleep(heartbeat_timeout);
    tokio::pin!(silence);

    loop {
        tokio::select! {
            line = lines.next_line() => match line {
                Ok(Some(line)) => {
                    match serde_json::from_str::<PeerMessage>(&line) {
                        Ok(message) => match message {
                            PeerMessage::Hello { node } if remote_node.is_none() && valid_node_name(&node) => {
                                silence.as_mut().reset(tokio::time::Instant::now() + heartbeat_timeout);
                                remote_node = Some(node.clone());
                                if events.send(SessionEvent::Connected {
                                    id,
                                    node,
                                    direction,
                                    tx: out_tx.clone(),
                                    shutdown: shutdown_tx.take().expect("hello is accepted only once"),
                                }).await.is_err() {
                                    break;
                                }
                            }
                            PeerMessage::Heartbeat if remote_node.is_some() => {
                                silence.as_mut().reset(tokio::time::Instant::now() + heartbeat_timeout);
                            }
                            message => {
                                if let Some(node) = &remote_node {
                                    silence.as_mut().reset(tokio::time::Instant::now() + heartbeat_timeout);
                                    if events.send(SessionEvent::Incoming { id, node: node.clone(), message }).await.is_err() {
                                        break;
                                    }
                                }
                            }
                        },
                        Err(e) => warn!("invalid node message: {e}"),
                    }
                }
                Ok(None) | Err(_) => break,
            },
            Some(message) = out_rx.recv() => {
                if write_message(&mut write, &message).await.is_err() {
                    break;
                }
            }
            _ = heartbeat.tick() => {
                if write_message(&mut write, &PeerMessage::Heartbeat).await.is_err() {
                    break;
                }
            }
            _ = &mut silence => {
                warn!(
                    "node heartbeat timed out: {}",
                    remote_node.as_deref().unwrap_or("unknown")
                );
                break;
            }
            _ = &mut shutdown_rx => break,
        }
    }

    if let Some(node) = remote_node {
        let _ = events.send(SessionEvent::Disconnected { id, node }).await;
    }
}

async fn write_message<W: AsyncWrite + Unpin>(
    write: &mut W,
    message: &PeerMessage,
) -> std::io::Result<()> {
    let mut encoded = serde_json::to_vec(message)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    encoded.push(b'\n');
    write.write_all(&encoded).await?;
    write.flush().await
}

impl WireVal {
    fn from_val(value: Val) -> Result<Self> {
        Ok(match value {
            Val::Nil => Self::Nil,
            Val::Bool(value) => Self::Bool(value),
            Val::Int(value) => Self::Int(value),
            Val::String(value) => Self::String(value),
            Val::Symbol(value) => Self::Symbol(value),
            Val::Keyword(value) => Self::Keyword(value),
            Val::List(values) => Self::List(
                values
                    .into_iter()
                    .map(Self::from_val)
                    .collect::<Result<_>>()?,
            ),
            Val::Error(value) => Self::Error(value),
            Val::Ref(value) => Self::Ref(value),
            Val::Extern(Extern::ProcessId(pid)) => Self::Process(pid),
            Val::Lambda(_)
            | Val::NativeFn(_)
            | Val::NativeAsyncFn(_)
            | Val::Bytecode(_)
            | Val::Extern(Extern::RequestId(_)) => {
                return Err(Error::RegistryError(
                    "message contains a value that cannot cross nodes".to_string(),
                ))
            }
        })
    }

    fn into_val(self) -> Result<Val> {
        Ok(match self {
            Self::Nil => Val::Nil,
            Self::Bool(value) => Val::Bool(value),
            Self::Int(value) => Val::Int(value),
            Self::String(value) => Val::String(value),
            Self::Symbol(value) => Val::Symbol(value),
            Self::Keyword(value) => Val::Keyword(value),
            Self::List(values) => Val::List(
                values
                    .into_iter()
                    .map(Self::into_val)
                    .collect::<Result<_>>()?,
            ),
            Self::Error(value) => Val::Error(value),
            Self::Ref(value) => Val::Ref(value),
            Self::Process(pid) => Val::Extern(Extern::ProcessId(pid)),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{duplex, split};
    use tokio::time::timeout;

    #[test]
    fn node_endpoints_have_explicit_transports_and_optional_ports() {
        assert_eq!(
            NodeEndpoint::parse("tcp://node"),
            Some(NodeEndpoint::Tcp("node:8773".to_string()))
        );
        assert_eq!(
            NodeEndpoint::parse("tcp://127.0.0.1:4567"),
            Some(NodeEndpoint::Tcp("127.0.0.1:4567".to_string()))
        );
        assert_eq!(
            NodeEndpoint::parse("ssh://node"),
            Some(NodeEndpoint::Ssh {
                host: "node".to_string(),
                port: 8773,
            })
        );
        assert_eq!(
            NodeEndpoint::parse("ssh://user@node:4567"),
            Some(NodeEndpoint::Ssh {
                host: "user@node".to_string(),
                port: 4567,
            })
        );
        assert_eq!(NodeEndpoint::parse("node"), None);
        assert_eq!(NodeEndpoint::parse("ssh://-oProxyCommand=bad"), None);
        assert_eq!(NodeEndpoint::parse("tcp://node:not-a-port"), None);
    }

    #[test]
    fn ssh_links_forward_stdio_to_the_remote_vrs_port() {
        let command = ssh_command("node", 4567);
        assert_eq!(command.as_std().get_program(), "ssh");
        assert_eq!(
            command
                .as_std()
                .get_args()
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect::<Vec<_>>(),
            ["-T", "-o", "BatchMode=yes", "-W", "127.0.0.1:4567", "node"]
        );
    }

    #[test]
    fn wire_values_preserve_process_addresses() {
        let value = Val::List(vec![
            Val::keyword("reply_to"),
            Val::Extern(Extern::ProcessId(ProcessId::new("alpha", 3))),
        ]);
        let wire = WireVal::from_val(value).unwrap();
        let remote = wire.into_val().unwrap();
        assert_eq!(
            remote,
            Val::List(vec![
                Val::keyword("reply_to"),
                Val::Extern(Extern::ProcessId(ProcessId::new("alpha", 3))),
            ])
        );
    }

    #[tokio::test]
    async fn silent_session_disconnects_after_heartbeat_timeout() {
        let (local, remote) = duplex(4096);
        let (local_read, local_write) = split(local);
        let (remote_read, mut remote_write) = split(remote);
        let mut remote_lines = BufReader::new(remote_read).lines();
        let (events_tx, mut events_rx) = mpsc::channel(8);

        let session = tokio::spawn(run_session_with_heartbeat(
            local_read,
            local_write,
            "alpha".to_string(),
            Direction::Outgoing,
            events_tx,
            Duration::from_millis(10),
            Duration::from_millis(40),
        ));

        let hello = remote_lines.next_line().await.unwrap().unwrap();
        assert!(matches!(
            serde_json::from_str::<PeerMessage>(&hello).unwrap(),
            PeerMessage::Hello { node } if node == "alpha"
        ));
        write_message(
            &mut remote_write,
            &PeerMessage::Hello {
                node: "beta".to_string(),
            },
        )
        .await
        .unwrap();

        let _shutdown = match events_rx.recv().await.unwrap() {
            SessionEvent::Connected { node, shutdown, .. } if node == "beta" => shutdown,
            event => panic!("expected beta to connect, got {event:?}"),
        };
        let heartbeat = timeout(Duration::from_millis(100), remote_lines.next_line())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert!(matches!(
            serde_json::from_str::<PeerMessage>(&heartbeat).unwrap(),
            PeerMessage::Heartbeat
        ));
        assert!(matches!(
            timeout(Duration::from_millis(200), events_rx.recv())
                .await
                .unwrap()
                .unwrap(),
            SessionEvent::Disconnected { node, .. } if node == "beta"
        ));
        session.await.unwrap();
    }
}
