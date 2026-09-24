#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::{Context, Result};
use clap::Parser;
use lyric::Form;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
#[cfg(test)]
mod client_tests;
mod protocol;
use tauri::{async_runtime::JoinHandle, Emitter, Manager, PhysicalPosition, WebviewWindow};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use tokio::{
    net::UnixStream,
    sync::{mpsc, oneshot},
};
use tracing::error;
use vrs::{Connection, Response};

#[cfg(target_os = "macos")]
use tauri::ActivationPolicy;

struct State {
    client: Client,
}

struct PreviewMode(bool);

#[derive(Parser)]
struct Args {
    /// Unix socket for vrsd; defaults to the release/debug-specific socket.
    #[arg(long)]
    socket: Option<PathBuf>,
    /// Keep the palette open on focus loss without registering a global shortcut.
    #[arg(long)]
    preview: bool,
}

impl State {
    fn new(client: Client) -> Self {
        Self { client }
    }
}

/// Tauri-client bridge
struct Client {
    socket: PathBuf,
    task: Option<JoinHandle<anyhow::Result<()>>>,
    hdl_tx: Option<mpsc::Sender<Cmd>>,
}

enum Cmd {
    Request(Form, oneshot::Sender<Result<Response>>),
}

impl Client {
    fn new(socket: PathBuf) -> Self {
        Self {
            socket,
            task: None,
            hdl_tx: None,
        }
    }

    fn start(&mut self, notify: impl Fn(&str) + Send + 'static) -> Result<()> {
        if self.task.is_some() {
            panic!("Client is unexpectedly started twice");
        }

        let (tx, mut rx) = mpsc::channel(32);
        self.hdl_tx = Some(tx);
        let socket = self.socket.clone();

        self.task = Some(tauri::async_runtime::spawn(async move {
            loop {
                let connection = async {
                    let stream = UnixStream::connect(&socket).await?;
                    let client = Arc::new(vrs::Client::new(Connection::new(stream)));
                    let subscription = client.subscribe(vrs::KeywordId::from("vrsjmp")).await?;
                    // Requests and subscriptions share this one socket. This
                    // round trip also orders startup after subscription setup.
                    client.request(Form::Nil).await?;
                    Ok::<_, anyhow::Error>((client, subscription))
                }
                .await;
                let (client, mut subscription) = match connection {
                    Ok(connection) => connection,
                    Err(error) => {
                        tracing::warn!("VRS connection unavailable: {error}");
                        let retry = tokio::time::sleep(Duration::from_secs(1));
                        tokio::pin!(retry);
                        loop {
                            tokio::select! {
                                _ = &mut retry => break,
                                command = rx.recv() => match command {
                                    Some(Cmd::Request(_, reply)) => {
                                        let _ = reply.send(Err(anyhow::anyhow!("VRS connection unavailable: {error}")));
                                    }
                                    None => return Ok(()),
                                }
                            }
                        }
                        continue;
                    }
                };
                notify("vrs-connected");
                let mut requests = tokio::task::JoinSet::new();
                loop {
                    tokio::select! {
                        command = rx.recv() => match command {
                            Some(Cmd::Request(form, reply)) => {
                                let client = client.clone();
                                requests.spawn(async move {
                                    let result = client.request(form).await.map_err(anyhow::Error::from);
                                    let _ = reply.send(result);
                                });
                            }
                            None => return Ok(()),
                        },
                        event = subscription.recv() => match event {
                            Ok(Form::Keyword(event)) if event.as_str() == "show" => notify("show-palette"),
                            Ok(Form::Keyword(event)) if event.as_str() == "config_changed" => notify("vrs-config-changed"),
                            Ok(_) => {},
                            Err(error) => {
                                tracing::warn!("VRS subscription interrupted: {error}");
                                break;
                            }
                        },
                        _ = client.closed() => break,
                        _ = requests.join_next(), if !requests.is_empty() => {},
                    }
                }
                requests.abort_all();
                client.shutdown().await;
            }
        }));
        Ok(())
    }

    async fn request(&self, form: lyric::Form) -> Result<Response> {
        let hdl_tx = self.hdl_tx.as_ref().context("Client task is not started")?;
        let (resp_tx, resp_rx) = oneshot::channel();
        hdl_tx.send(Cmd::Request(form, resp_tx)).await?;
        resp_rx.await.context("Failed to receive response")?
    }
}

async fn evaluate(state: &State, form: Form) -> Result<Form> {
    state
        .client
        .request(form)
        .await?
        .contents
        .map_err(anyhow::Error::from)
}

#[tauri::command]
async fn set_query(
    renderer: String,
    args: String,
    query: String,
    state: tauri::State<'_, State>,
) -> Result<Vec<protocol::Item>, String> {
    let result = async {
        let request = protocol::query_request(&renderer, &args, &query)?;
        protocol::items(evaluate(&state, request).await?)
    }
    .await;
    result.map_err(|error: anyhow::Error| {
        error!("Error requesting items: {error}");
        error.to_string()
    })
}

#[tauri::command]
async fn dispatch(
    form: String,
    state: tauri::State<'_, State>,
) -> Result<protocol::Action, String> {
    let result = async {
        let request = protocol::action_request(&form)?;
        // A terminal evaluates requests in order. Keep long-running actions
        // off the navigation connection so hiding or reopening stays responsive.
        let stream = UnixStream::connect(&state.client.socket).await?;
        let client = vrs::Client::new(Connection::new(stream));
        protocol::action(client.request(request).await?.contents?)
    }
    .await;
    result.map_err(|error: anyhow::Error| {
        error!("Error dispatching item: {error}");
        error.to_string()
    })
}

#[tauri::command]
async fn root_page(state: tauri::State<'_, State>) -> Result<protocol::Action, String> {
    // The service chooses Home or a pending input page before the window opens.
    let result =
        async { protocol::action(evaluate(&state, protocol::root_request()).await?) }.await;
    result.map_err(|error: anyhow::Error| error.to_string())
}

#[tauri::command]
async fn ui_config(state: tauri::State<'_, State>) -> Result<protocol::UiConfig, String> {
    let result = async {
        protocol::ui_config(
            evaluate(&state, protocol::service_request("get_ui_config", vec![])).await?,
        )
    }
    .await;
    result.map_err(|error: anyhow::Error| error.to_string())
}

#[tauri::command]
fn preview_mode(preview: tauri::State<'_, PreviewMode>) -> bool {
    preview.0
}

#[tauri::command]
fn show(app: tauri::AppHandle, preview: tauri::State<'_, PreviewMode>) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        if !window.is_visible().map_err(|error| error.to_string())? {
            center_in_primary_monitor(&window);
            if preview.0 {
                window
                    .set_focusable(false)
                    .map_err(|error| error.to_string())?;
            }
            window.show().map_err(|error| error.to_string())?;
        }
        if !preview.0 {
            window.set_focus().map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

#[tauri::command]
fn hide(app: tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
    }
    #[cfg(target_os = "macos")]
    let _ = app.hide();
}

#[tauri::command]
fn on_blur(app: tauri::AppHandle, preview: tauri::State<'_, PreviewMode>) {
    if !preview.0
        && app
            .get_webview_window("main")
            .is_some_and(|window| !window.is_focused().unwrap_or(false))
    {
        hide(app);
    }
}

#[cfg(target_os = "macos")]
fn configure_palette_spaces(window: &WebviewWindow) -> Result<()> {
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSWindow, NSWindowCollectionBehavior};

    let _main_thread = MainThreadMarker::new()
        .context("Palette Spaces behavior must be configured on the main thread")?;
    let native_window = window.ns_window()?;
    // SAFETY: Tauri supplies the NSWindow owned by this live window. Setup runs
    // on the main thread (checked above), and the pointer is only borrowed here.
    let native_window = unsafe { native_window.cast::<NSWindow>().as_ref() }
        .context("Palette has no native macOS window")?;
    // Set this before the first show(), which can already make the window key.
    // Activation should bring the palette here instead of switching Spaces.
    native_window.setCollectionBehavior(
        native_window.collectionBehavior() | NSWindowCollectionBehavior::MoveToActiveSpace,
    );
    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    let socket = args.socket.unwrap_or_else(vrs::runtime_socket);

    let context = tauri::generate_context!();
    tauri::Builder::default()
        .manage(PreviewMode(args.preview))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _, event| {
                    if event.state == ShortcutState::Pressed {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.emit("toggle-palette", ());
                        }
                    }
                })
                .build(),
        )
        .setup(move |app| {
            let window = app.get_webview_window("main").unwrap();
            #[cfg(target_os = "macos")]
            configure_palette_spaces(&window)?;
            let notifications = window.clone();
            let mut client = Client::new(socket.clone());
            client.start(move |event| {
                let _ = notifications.emit(event, ());
            })?;
            app.manage(State::new(client));

            #[cfg(target_os = "macos")]
            app.set_activation_policy(ActivationPolicy::Accessory);

            let binding = if cfg!(debug_assertions) {
                "CMD+CTRL+SHIFT+SPACE" // debug
            } else {
                "CMD+SPACE" // release
            };

            if !args.preview {
                app.global_shortcut().register(binding)?;
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            set_query,
            dispatch,
            root_page,
            ui_config,
            preview_mode,
            show,
            hide,
            on_blur
        ])
        .run(context)
        .expect("error while running tauri application");

    Ok(())
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
    let x = monitor_pos.x + (monitor_size.width as i32 - window_size.width as i32).max(0) / 2;
    let y = monitor_pos.y + (monitor_size.height as i32 - window_size.height as i32).max(0) / 2;
    if let Err(e) = window.set_position(PhysicalPosition::new(x, y)) {
        tracing::error!("Failed to set position - {e}");
    }
}
