#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// TODO: Major Cleanup for Cowboy Coding

use anyhow::{Context, Error, Result};
use lyric::{kwargs, Form};
use nucleo_matcher::{
    pattern::{CaseMatching, Normalization, Pattern},
    Matcher,
};
use serde_json::json;
use std::sync::Mutex;
use tauri::{async_runtime::JoinHandle, AppHandle, Manager, PhysicalPosition, WebviewWindow};
use tauri_plugin_global_shortcut::GlobalShortcutExt;
use tokio::{
    net::UnixStream,
    sync::{mpsc, oneshot},
};
use tracing::error;
use vrs::{Connection, KeywordId, Response, Val};

#[cfg(target_os = "macos")]
use tauri::ActivationPolicy;

#[cfg(target_os = "macos")]
use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial, NSVisualEffectState};

struct State {
    matcher: Mutex<Matcher>,
    client: Client,
}

impl State {
    fn new(client: Client) -> Self {
        Self {
            client,
            matcher: Mutex::new(Matcher::default()),
        }
    }
}

/// Tauri-client bridge
struct Client {
    task: Option<JoinHandle<anyhow::Result<()>>>,
    hdl_tx: Option<mpsc::Sender<Cmd>>,
}

enum Cmd {
    Request(Form, oneshot::Sender<Response>),
}

impl Client {
    fn new() -> Self {
        Self {
            task: None,
            hdl_tx: None,
        }
    }

    fn start(&mut self) -> Result<()> {
        if self.task.is_some() {
            panic!("Client is unexpectedly started twice");
        }

        let (tx, mut rx) = mpsc::channel(32);
        self.hdl_tx = Some(tx);

        self.task = Some(tauri::async_runtime::spawn(async move {
            let socket = vrs::runtime_socket();
            let conn = UnixStream::connect(socket)
                .await
                .map(Connection::new)
                .with_context(|| "Failed to connect to vrsd socket")?;
            let client = vrs::Client::new(conn);

            while let Some(cmd) = rx.recv().await {
                match cmd {
                    Cmd::Request(f, resp_tx) => {
                        let res = client.request(f).await?;
                        let _ = resp_tx.send(res);
                    }
                }
            }

            Ok::<(), Error>(())
        }));
        Ok(())
    }

    fn request(&self, form: lyric::Form) -> Result<Response> {
        tauri::async_runtime::block_on(async {
            let hdl_tx = self.hdl_tx.clone().expect("Client task is not started");
            let (resp_tx, resp_rx) = oneshot::channel();
            hdl_tx.send(Cmd::Request(form, resp_tx)).await?;
            let res = resp_rx
                .await
                .with_context(|| "Failed to receive response")?;

            Ok(res)
        })
    }
}

#[tauri::command]
fn set_query(query: &str, state: tauri::State<State>) -> Vec<serde_json::Value> {
    let mut matcher = state.matcher.lock().unwrap();

    // TODO: Contents of `query` should be escaped
    let request = match Form::from_expr(&format!(
        "(begin (bind_srv :vrsjmp) (get_items \"{}\"))",
        query
    )) {
        Ok(f) => f,
        Err(lyric::Error::IncompleteExpression(_)) => return vec![],
        Err(e) => {
            error!("Invalid form for user query - {e}");
            return vec![];
        }
    };

    let response = state.client.request(request).unwrap();

    let items = match response.contents.unwrap() {
        Form::List(items) => items.iter().map(|i| i.to_string()).collect(),
        e => {
            error!("Received unexpected response from client - {e}");
            vec![]
        }
    };

    let matches = Pattern::parse(query, CaseMatching::Smart, Normalization::Smart)
        .match_list(items, &mut matcher);

    let mut result = vec![];
    for (i, _) in matches {
        // TODO: Make kwarg extraction more ergonomic (?)
        let form_args = match Val::from(Form::from_expr(&i).unwrap()) {
            Val::List(l) => l,
            _ => panic!("Unexpected format - not a list"),
        };
        let title = match kwargs::get(&form_args, &KeywordId::from("title")).unwrap() {
            Val::String(t) => t,
            _ => panic!("Unexpected format - not a list"),
        };

        result.push(json!({
            "title": title,
            "on_click": format!("{}", i),
        }))
    }
    result
}

#[tauri::command]
fn dispatch(form: &str, state: tauri::State<State>, app: tauri::AppHandle) {
    let client = &state.client;

    let dispatch = Form::from_expr(&format!("(on_click (quote {}))", form)).unwrap();
    if let Err(e) = client.request(dispatch) {
        error!("Error dispatching request - {e}");
    }

    let window = app.get_webview_window("main").unwrap();
    let _ = window.hide();

    #[cfg(target_os = "macos")]
    let _ = app.hide();
}

#[tauri::command]
fn on_blur(app: tauri::AppHandle) {
    if cfg!(debug_assertions) {
        return; // skip hiding on debug
    }
    let window = app.get_webview_window("main").unwrap();
    let _ = window.hide();
    #[cfg(target_os = "macos")]
    let _ = app.hide();
}

fn main() -> Result<()> {
    let mut client = Client::new();
    client
        .start()
        .with_context(|| "Failed to start vrs client")?;

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(State::new(client))
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();

            #[cfg(target_os = "macos")]
            app.set_activation_policy(ActivationPolicy::Accessory);

            #[cfg(target_os = "macos")]
            apply_vibrancy(
                &window,
                NSVisualEffectMaterial::HudWindow,
                Some(NSVisualEffectState::Active),
                Some(16.0),
            )
            .expect("Unsupported platform! 'apply_vibrancy' is only supported on macOS");

            let binding = if cfg!(debug_assertions) {
                "CMD+CTRL+SHIFT+SPACE" // debug
            } else {
                "CMD+SPACE" // release
            };

            let window_clone = window.clone();
            let app_handle = app.handle();
            let app_handle_clone = app_handle.clone();
            app_handle
                .global_shortcut()
                .on_shortcut(binding, move |_app, _, _| {
                    toggle_window_visibility(&window_clone, &app_handle_clone);
                })?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![set_query, dispatch, on_blur])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    Ok(())
}

fn toggle_window_visibility(window: &WebviewWindow, app_handle: &AppHandle) {
    let visible = window
        .is_visible()
        .expect("should retrieve window visibility");
    if visible {
        #[cfg(target_os = "macos")]
        let _ = app_handle.hide();
        let _ = window.hide();
    } else {
        center_in_primary_monitor(&window);
        let _ = window.set_focus();
    }
}

fn center_in_primary_monitor(window: &WebviewWindow) {
    let primary_monitor = match window.primary_monitor() {
        Ok(Some(m)) => m,
        Err(e) => {
            tracing::error!("Failed to get primary monitor - {e}");
            return;
        }
        Ok(None) => {
            tracing::warn!("No primary monitor found");
            return;
        }
    };

    let window_size = match window.inner_size() {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("Failed to fetch inner size - {e}");
            return;
        }
    };

    let monitor_pos = primary_monitor.position();
    let monitor_size = primary_monitor.size();
    let x = monitor_pos.x + (monitor_size.width / 2 - window_size.width / 2) as i32;
    let y = monitor_pos.y + (monitor_size.height / 2 - window_size.height / 2) as i32;
    if let Err(e) = window.set_position(PhysicalPosition::new(x, y)) {
        tracing::error!("Failed to set position - {e}");
    }
}
