//! REPL for vrsctl
use anyhow::Result;

use lyric::Form;
use std::path::PathBuf;
use vrs::Client;

use crate::editor::{self, Editor};
use rustyline::error::ReadlineError;

/// Entrypoint for running REPL.
/// Returns Err if REPL terminated with error
pub(crate) async fn run(client: &mut Client) -> Result<()> {
    let mut rl = editor::editor()?;
    // let mut printer = rl.create_external_printer()?;
    let history = history_file();

    load_history(&mut rl, &history);

    loop {
        match rl.readline("vrs> ") {
            Ok(line) => {
                let _ = rl.add_history_entry(line.as_str());
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
                        Ok(Form::RawString(s)) => println!("{}", s),
                        Ok(c) => println!("{}", c),
                        Err(e) => eprintln!("{}", e),
                    },
                    Err(e) => {
                        eprintln!("{}", e);
                    }
                }
            }
            Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => break,
            Err(err) => {
                println!("Error: {:?}", err);
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
