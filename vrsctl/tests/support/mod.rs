use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{ensure, Context, Result};
use expectrl::process::unix::{Signal, WaitStatus};
use expectrl::session::OsSession;
use expectrl::{Eof, Expect, Session};
use tempfile::TempDir;

pub const VRSCTL: &str = env!("CARGO_BIN_EXE_vrsctl");
pub const TIMEOUT: Duration = Duration::from_secs(5);

pub struct TestRuntime {
    daemon: Child,
    pub directory: TempDir,
    socket: PathBuf,
}

impl TestRuntime {
    pub fn new() -> Result<Self> {
        // Keep Unix socket paths below macOS's short limit.
        let directory = tempfile::Builder::new()
            .prefix("vrs-terminal-")
            .tempdir_in("/tmp")?;
        let socket = directory.path().join("runtime.sock");
        let init = directory.path().join("init.ll");
        fs::write(
            &init,
            "(defn! start_child (name value)
  (defn! read_value () value)
  (spawn_srv! name :interface '(read_value)))
(spawn_srv! :test_factory :interface '(start_child))
",
        )?;
        let log_path = directory.path().join("daemon.log");
        let log = File::create(&log_path)?;
        let binary = Path::new(VRSCTL).with_file_name("vrsd");
        let daemon = Command::new(&binary)
            .args(["--node", "format-test", "--node-port", "0", "--socket"])
            .arg(&socket)
            .arg("--init")
            .arg(init)
            .stdin(Stdio::null())
            .stdout(log.try_clone()?)
            .stderr(log)
            .spawn()
            .with_context(|| {
                format!(
                    "could not start {}; run cargo build -p vrsd with the same profile and target directory as the tests",
                    binary.display()
                )
            })?;
        let mut runtime = Self {
            daemon,
            directory,
            socket,
        };
        let deadline = Instant::now() + Duration::from_secs(10);
        while !runtime.socket.exists() {
            ensure!(
                runtime.daemon.try_wait()?.is_none() && Instant::now() < deadline,
                "test daemon failed to start:\n{}",
                fs::read_to_string(&log_path)?
            );
            thread::sleep(Duration::from_millis(20));
        }
        Ok(runtime)
    }

    pub fn command(&self, args: &[&str]) -> Command {
        let mut command = Command::new(VRSCTL);
        command.arg("--socket").arg(&self.socket).args(args);
        command
    }

    pub fn pipe(&self, args: &[&str]) -> String {
        self.pipe_input(args, None)
    }

    pub fn pipe_input(&self, args: &[&str], source: Option<&str>) -> String {
        let mut command = assert_cmd::Command::from_std(self.command(args));
        command.timeout(TIMEOUT);
        if let Some(source) = source {
            command.write_stdin(source);
        }
        let assertion = command.assert().success();
        String::from_utf8(assertion.get_output().stdout.clone()).expect("non-UTF-8 CLI output")
    }

    pub fn terminal(&self, args: &[&str], stdin: bool, columns: u16) -> Result<Terminal> {
        let mut command = self.command(args);
        command.env("TERM", "xterm-256color");
        if !stdin {
            command.stdin(Stdio::null());
        }
        let mut terminal = Terminal {
            session: Session::spawn(command)?,
            reaped: false,
        };
        terminal.session.set_expect_timeout(Some(TIMEOUT));
        terminal.resize(columns)?;
        Ok(terminal)
    }

    pub fn emacs_command(&self) -> String {
        // Emacs runs this through a shell; quote paths independently of arguments.
        let quote = |value: &str| format!("'{}'", value.replace('\'', "'\\''"));
        format!(
            "{} --socket {}",
            quote(VRSCTL),
            quote(self.socket.to_str().expect("non-UTF-8 test socket"))
        )
    }
}

impl Drop for TestRuntime {
    fn drop(&mut self) {
        let _ = self.daemon.kill();
        let _ = self.daemon.wait();
    }
}

pub struct Terminal {
    pub session: OsSession,
    reaped: bool,
}

impl Terminal {
    pub fn expect(&mut self, expected: &str) -> Result<()> {
        // The PTY translates output newlines to CRLF. ANSI sequences surrounding
        // prompts/results stay in expectrl's buffer and do not affect matching.
        self.session
            .expect(expected.replace('\n', "\r\n"))
            .with_context(|| format!("expected {expected:?} in terminal output"))?;
        Ok(())
    }

    pub fn resize(&mut self, columns: u16) -> Result<()> {
        self.session
            .get_process_mut()
            .set_window_size(columns, 24)?;
        Ok(())
    }

    pub fn assert_alive(&mut self) -> Result<()> {
        let status = self.session.get_process().status()?;
        self.reaped = status != WaitStatus::StillAlive;
        ensure!(!self.reaped, "terminal process exited early: {status:?}");
        Ok(())
    }

    pub fn success(&mut self) -> Result<()> {
        self.session.expect(Eof)?;
        let deadline = Instant::now() + TIMEOUT;
        loop {
            let status = self.session.get_process().status()?;
            if status != WaitStatus::StillAlive {
                self.reaped = true;
                ensure!(
                    matches!(status, WaitStatus::Exited(_, 0)),
                    "terminal process failed: {status:?}"
                );
                return Ok(());
            }
            ensure!(Instant::now() < deadline, "terminal process did not exit");
            thread::sleep(Duration::from_millis(20));
        }
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        if !self.reaped {
            // Stop REPLs before normal exit can save the user's history. Also
            // reap children when an assertion fails or a subscription stays open.
            let process = self.session.get_process_mut();
            let _ = process.kill(Signal::SIGKILL);
            let _ = process.wait();
        }
    }
}
