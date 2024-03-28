//! REPL for vrsctl
use anyhow::Result;

use std::path::PathBuf;
use vrs::Client;

use crate::editor::{self, Editor};
use crate::output::Output;
use rustyline::error::ReadlineError;

/// Entrypoint for running REPL.
/// Returns Err if REPL terminated with error
pub(crate) async fn run(client: &Client, output: &Output) -> Result<()> {
    let mut rl = editor::editor()?;
    let history = history_file();

    load_history(&mut rl, &history);

    loop {
        let line = match rl.readline("vrs> ") {
            Ok(line) => {
                let _ = rl.add_history_entry(line.as_str());
                line
            }
            Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => break,
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        };
        match lyric::parse(&line) {
            Ok(_) => (),
            Err(e) => {
                eprintln!("{}", e);
                continue;
            }
        };
        // TODO: Interrupt request with ctrl-c?
        match client
            .request(lyric::source::request(&line, "<repl>", 1, 1))
            .await
        {
            Ok(resp) => match resp.contents {
                Ok(c) => {
                    // readline has returned and restored the terminal. Results
                    // are synchronous, so no external-printer queue is needed.
                    output.write(&mut std::io::stdout(), &c, "")?;
                }
                Err(e) => eprintln!("{}", e),
            },
            Err(e) => {
                eprintln!("{}", e);
                break;
            }
        }
    }

    save_history(&mut rl, &history);
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
