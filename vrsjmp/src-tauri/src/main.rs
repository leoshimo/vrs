#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde_json::json;
use tauri::{GlobalShortcutManager, Manager};

#[cfg(target_os = "macos")]
use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial, NSVisualEffectState};

#[tauri::command]
fn set_query(query: &str) -> Vec<serde_json::Value> {
    let mut result = vec![];
    for i in 0..=10 {
        result.push(json!({
            "title": format!("{} {}", query, i),
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
