//! REPL for vrsctl
use anyhow::Result;

use lyric::Form;
use std::{path::PathBuf, thread};
use tokio::sync::mpsc;
use vrs::Client;

use crate::editor::{self, Editor};
use rustyline::{error::ReadlineError, ExternalPrinter};

/// Entrypoint for running REPL.
/// Returns Err if REPL terminated with error
pub(crate) async fn run(client: &Client) -> Result<()> {
    let mut rl = editor::editor()?;
    let mut printer = rl.create_external_printer()?;
    let history = history_file();
    let (line_tx, mut line_rx) = mpsc::channel(32);

    // Uses separate thread for rustyline - rustyline is not async
    thread::spawn(move || loop {
        load_history(&mut rl, &history);
        match rl.readline("vrs> ") {
            Ok(line) => {
                let _ = rl.add_history_entry(line.as_str());
                let _ = line_tx.blocking_send(line);
            }
            Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => break,
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
        save_history(&mut rl, &history);
    });

    loop {
        let line = match line_rx.recv().await {
            Some(l) => l,
            None => break, // rustyline exited
        };
        let f = match lyric::parse(&line) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("{}", e);
                continue;
            }
        };
        match client.request(f).await {
            Ok(resp) => match resp.contents {
                // TODO: Bringup different formats for clients - e.g. REPL should use text format only
                Ok(Form::RawString(s)) => {
                    printer.print(format!("{s}\n"))?;
                }
                Ok(c) => printer.print(format!("{c}\n"))?,
                Err(e) => eprintln!("{}", e),
            },
            Err(e) => {
                eprintln!("{}", e);
                break;
            }
        }
    }
    client.shutdown().await;

    Ok(())
}

/// Path to file to use for history
fn history_file() -> Option<PathBuf> {
    let dir = dirs::data_local_dir()
        .or_else(dirs::data_dir)
        .or_else(dirs::home_dir)?;
    Some(dir.as_path().join(".vrsctl_history"))
}

fn load_history(rl: &mut Editor, history: &Option<PathBuf>) {
    if let Some(history) = history {
        if let Err(e) = rl.load_history(&history) {
            eprintln!("Failed to load {} - {}", history.to_string_lossy(), e);
        }
    }
}

fn save_history(rl: &mut Editor, history: &Option<PathBuf>) {
    if let Some(history) = history {
        if let Err(e) = rl.save_history(&history) {
            eprintln!("Failed to save {} - {}", history.to_string_lossy(), e);
        }
    }
}
