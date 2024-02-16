#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Mutex;

use nucleo_matcher::{
    pattern::{CaseMatching, Normalization, Pattern},
    Matcher,
};
use serde_json::json;
use tauri::{GlobalShortcutManager, Manager};

#[cfg(target_os = "macos")]
use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial, NSVisualEffectState};

struct State {
    matcher: Mutex<Matcher>,
}

impl State {
    fn new() -> Self {
        Self {
            matcher: Mutex::new(Matcher::default()),
        }
    }
}

#[tauri::command]
fn set_query(query: &str, state: tauri::State<State>) -> Vec<serde_json::Value> {
    let mut matcher = state.matcher.lock().unwrap();

    // TODO: Integrate with Client
    let items = vec![
        "Open File",
        "New Document",
        "Save",
        "Save As...",
        "Close Window",
        "Undo",
        "Redo",
        "Cut",
        "Copy",
        "Paste",
        "Find",
        "Replace",
        "Go To Line",
        "Select All",
        "Preferences",
        "Toggle Fullscreen",
        "Zoom In",
        "Zoom Out",
        "Help",
        "Exit",
    ];

    let matches = Pattern::parse(query, CaseMatching::Smart, Normalization::Smart)
        .match_list(items, &mut matcher);

    let mut result = vec![];
    for (i, _) in matches {
        result.push(json!({
            "title": format!("{}", i),
            "on_click": format!("(send {})", i),
        }))
    }
    result
}

#[tauri::command]
fn dispatch(form: &str) {
    println!("received {}", form);
}

fn main() {
    tauri::Builder::default()
        .manage(State::new())
        .setup(|app| {
            let window = app.get_window("main").unwrap();

            #[cfg(target_os = "macos")]
            apply_vibrancy(
                &window,
                NSVisualEffectMaterial::HudWindow,
                Some(NSVisualEffectState::Active),
                Some(16.0),
            )
            .expect("Unsupported platform! 'apply_vibrancy' is only supported on macOS");

            let mut shortcuts = app.global_shortcut_manager();
            shortcuts
                .register("CMD+CTRL+SPACE", move || {
                    let visible = window
                        .is_visible()
                        .expect("should retrieve window visibility");
                    if visible {
                        let _ = window.hide();
                    } else {
                        let _ = window.set_focus();
                    }
                })
                .unwrap();

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![set_query, dispatch])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
