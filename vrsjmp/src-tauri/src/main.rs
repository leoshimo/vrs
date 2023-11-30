#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[tauri::command]
fn eval(input: &str) -> String {
    format!("eval({}) = TODO", input)
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![eval])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
