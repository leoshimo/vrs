#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde_json::json;

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
        .invoke_handler(tauri::generate_handler![set_query, dispatch])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
