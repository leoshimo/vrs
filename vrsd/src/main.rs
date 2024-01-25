use anyhow::{Context, Result};
use clap::Parser;
use std::path::{Path, PathBuf};
use tokio::net::UnixListener;
use tracing::{error, info};
use vrs::{Connection, ProcessResult, Program, Runtime};

#[derive(Debug, Parser)]
#[command(about = "VRS runtime daemon")]
struct Args {
    /// Script evaluated inside a fresh runtime process before accepting clients.
    #[arg(long)]
    init: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();

    let path = vrs::runtime_socket();
    if path.exists() {
        std::fs::remove_file(&path)
            .with_context(|| format!("Failed to remove existing socket {}", path.display()))?;
    }

    let runtime = Runtime::new();

    if let Some(init_path) = args.init {
        run_init(&runtime, &init_path).await?;
    }

    let listener = UnixListener::bind(&path)
        .with_context(|| format!("Failed to start listener at {}", path.display()))?;

    loop {
        match listener.accept().await {
            Ok((conn, _addr)) => {
                info!("Connected to client: {:?}", conn);
                let conn = Connection::new(conn);
                runtime.handle_conn(conn).await?;
            }
            Err(e) => {
                error!("Unable to accept connections - {e}");
            }
        }
    }
}

async fn run_init(runtime: &Runtime, path: &Path) -> Result<()> {
    let source = tokio::fs::read_to_string(path)
        .await
        .with_context(|| format!("Failed to read init script {}", path.display()))?;
    let program = Program::from_script(&source)
        .with_context(|| format!("Failed to parse init script {}", path.display()))?;
    let exit = runtime
        .run(program)
        .await
        .with_context(|| format!("Failed to start init script {}", path.display()))?
        .join()
        .await
        .with_context(|| format!("Failed waiting for init script {}", path.display()))?;
    match exit.status {
        Ok(ProcessResult::Done(value)) => {
            info!("init complete: {} => {}", path.display(), value);
            Ok(())
        }
        Ok(ProcessResult::Cancelled) => {
            anyhow::bail!("Init script {} was cancelled", path.display())
        }
        Err(e) => Err(e).with_context(|| format!("Init script {} failed", path.display())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vrs::Val;

    #[tokio::test]
    async fn explicit_init_runs_as_a_script_and_services_survive() {
        let path = std::env::temp_dir().join(format!(
            "vrsd-init-test-{}-{}.ll",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(
            &path,
            "(defn ping () :pong)\n(spawn_srv :init_probe :interface '(ping))",
        )
        .unwrap();

        let runtime = Runtime::new();
        run_init(&runtime, &path).await.unwrap();
        let query = runtime
            .run(Program::from_expr("(begin (bind_srv :init_probe) (ping))").unwrap())
            .await
            .unwrap();
        assert_eq!(
            query.join().await.unwrap().status.unwrap(),
            ProcessResult::Done(Val::keyword("pong"))
        );

        std::fs::remove_file(path).unwrap();
    }
}
