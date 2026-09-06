#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// TODO: Major Cleanup for Cowboy Coding

use anyhow::{Context, Error, Result};
use clap::Parser;
use lyric::Form;
use std::path::{Path, PathBuf};
mod protocol;
use tauri::{async_runtime::JoinHandle, GlobalShortcutManager, Manager, PhysicalPosition, Window};
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

#[derive(Parser)]
struct Args {
    /// Unix socket for vrsd; defaults to the release/debug-specific socket.
    #[arg(long)]
    socket: Option<PathBuf>,
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

async fn request_once(
    client: &mut Option<vrs::Client>,
    socket: &Path,
    form: Form,
) -> Result<Response> {
    if client.is_none() {
        let conn = UnixStream::connect(socket)
            .await
            .map(Connection::new)
            .with_context(|| "Failed to connect to vrsd socket")?;
        *client = Some(vrs::Client::new(conn));
    }

    let result = client
        .as_ref()
        .expect("client should be connected")
        .request(form)
        .await;
    match result {
        Ok(response) => Ok(response),
        Err(error) => {
            *client = None;
            Err(error).with_context(|| "VRS request failed")
        }
    }
}

impl Client {
    fn new(socket: PathBuf) -> Self {
        Self {
            socket,
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
        let socket = self.socket.clone();

        self.task = Some(tauri::async_runtime::spawn(async move {
            let mut client = None;

            while let Some(cmd) = rx.recv().await {
                match cmd {
                    Cmd::Request(f, resp_tx) => {
                        let res = request_once(&mut client, &socket, f).await;
                        let _ = resp_tx.send(res);
                    }
                }
            }

            Ok::<(), Error>(())
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
        protocol::action(evaluate(&state, request).await?)
    }
    .await;
    result.map_err(|error: anyhow::Error| {
        error!("Error dispatching item: {error}");
        error.to_string()
    })
}

#[tauri::command]
async fn begin_interaction(state: tauri::State<'_, State>) -> Result<protocol::Action, String> {
    // The service owns context capture. The frontend requests this hook before
    // showing the window, so the originating application still has focus.
    let result = async {
        protocol::action(
            evaluate(
                &state,
                protocol::action_request("(:on_click (begin_interaction))")?,
            )
            .await?,
        )
    }
    .await;
    result.map_err(|error: anyhow::Error| error.to_string())
}

#[tauri::command]
fn show(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_window("main") {
        center_in_primary_monitor(&window);
        #[cfg(target_os = "macos")]
        app.show().map_err(|error| error.to_string())?;
        window.show().map_err(|error| error.to_string())?;
        window.set_focus().map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn hide(app: tauri::AppHandle) {
    if let Some(window) = app.get_window("main") {
        let _ = window.hide();
    }
    #[cfg(target_os = "macos")]
    let _ = app.hide();
}

#[tauri::command]
fn on_blur(app: tauri::AppHandle) {
    hide(app);
}

fn main() -> Result<()> {
    let args = Args::parse();
    let mut client = Client::new(args.socket.unwrap_or_else(vrs::runtime_socket));
    client
        .start()
        .with_context(|| "Failed to start vrs client")?;

    let context = tauri::generate_context!();
    #[cfg(target_os = "macos")]
    let context = {
        let mut context = context;
        if let Some(window) = context
            .config_mut()
            .tauri
            .windows
            .iter_mut()
            .find(|w| w.label == "main")
        {
            // AppKit owns the outline, clipping, and shadow as a single frame.
            // Overlay content fills it without a separate titlebar surface.
            window.decorations = true;
            window.transparent = false;
            window.title_bar_style = tauri::TitleBarStyle::Overlay;
            window.hidden_title = true;
        }
        context
    };

    tauri::Builder::default()
        .manage(State::new(client))
        .on_page_load(|_window, _| {
            #[cfg(target_os = "macos")]
            if let Err(error) = setup_native_frame(&_window) {
                error!("Failed to set up palette frame: {error}");
            }
        })
        .setup(|app| {
            // Tauri 1/tao dereferences a missing zoom button when maximizable is
            // false on a borderless macOS window (caught by debug Rust builds).
            // Leave it true in config; the palette remains non-resizable and
            // has no titlebar controls. TODO: Remove after upgrading Tauri.
            let window = app.get_window("main").unwrap();

            #[cfg(target_os = "macos")]
            app.set_activation_policy(ActivationPolicy::Accessory);

            let mut shortcuts = app.global_shortcut_manager();

            let binding = if cfg!(debug_assertions) {
                "CMD+CTRL+SHIFT+SPACE" // debug
            } else {
                "CMD+SPACE" // release
            };

            shortcuts
                .register(binding, move || {
                    let _ = window.emit("toggle-palette", ());
                })
                .unwrap();

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            set_query,
            dispatch,
            begin_interaction,
            show,
            hide,
            on_blur
        ])
        .run(context)
        .expect("error while running tauri application");

    Ok(())
}

#[cfg(target_os = "macos")]
#[allow(unexpected_cfgs)] // objc 0.2 macros refer to their historical cargo-clippy feature.
fn setup_native_frame(window: &Window) -> Result<()> {
    use cocoa::{
        appkit::{NSWindow, NSWindowButton},
        base::{id, YES},
    };
    use objc::{msg_send, sel, sel_impl};

    // Keep the native frame but hide its traffic-light controls. The palette
    // still closes with Escape or blur, not a conventional window close action.
    unsafe {
        let ns_window = window.ns_window()? as id;
        for kind in [
            NSWindowButton::NSWindowCloseButton,
            NSWindowButton::NSWindowMiniaturizeButton,
            NSWindowButton::NSWindowZoomButton,
        ] {
            let button = ns_window.standardWindowButton_(kind);
            if !button.is_null() {
                let _: () = msg_send![button, setHidden: YES];
            }
        }
    }
    window.eval("document.documentElement.classList.add('native-frame')")?;
    Ok(())
}

fn center_in_primary_monitor(window: &Window) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn palette_protocol_round_trip_and_error_recovery_over_a_real_connection() {
        let runtime = vrs::Runtime::new("gui-test");
        let mut forms = vec![vrs::Val::symbol("begin")];
        forms.extend(lyric::parse_script(include_str!("../../../scripts/vrsjmp.ll")).unwrap()
            .into_iter().filter(|form| matches!(form, Form::List(values) if
                values.first() == Some(&Form::symbol("defn")) || values.first() == Some(&Form::symbol("def"))))
            .map(vrs::Val::from));
        forms.extend(
            lyric::parse_script(
                r#"
            (defn get_rlist () '())
            (defn local_items () '((:title "Project Website" :on_click (open_url "https://searchable.example.test"))))
            (defn is_personal? () false)
            (defn feedbin_call (message) '((:title "Saved article" :url "https://example.test/")))
            (defn open_url (url) :opened)
            (def antinote_reads 0)
            (defn get_antinote_notes ()
              (set antinote_reads (+ antinote_reads 1))
              '((:antinote/note :id "note-1" :title "Meeting notes" :content "Meeting notes\nbody-only-needle" :modified "2026-09-05")))
            (defn open_antinote () :opened)
            (defn open_antinote_note (note)
              "Open Antinote Note"
              (interactive :antinote/note)
              (if (not? (eq? (get note :id) "note-1")) (error "Wrong note")))
            (defn get_windows () '((:os/window :id 7 :app "Safari" :title "Documentation")))
            (defn focus_window (window) "Focus Window" (interactive :os/window) :focused)
            (defn get_displays () '((:os/display :id 42 :index 2 :title "Display 2")))
            (defn move_window (window destination)
              "Move Window to Display"
              (interactive :os/window :os/display)
              (if (not? (eq? (list (get window :id) (get destination :index)) '(7 2)))
                (error "Wrong window or destination")))
            (set_entity_completions :os/window 'get_windows)
            (set_entity_completions :os/display 'get_displays)
            (set_entity_completions :antinote/note 'get_antinote_notes)
            (spawn_srv :vrsjmp :interface '(get_items on_click))
        "#,
            )
            .unwrap()
            .into_iter()
            .map(vrs::Val::from),
        );
        runtime
            .run(vrs::Program::from_val(vrs::Val::List(forms)).unwrap())
            .await
            .unwrap()
            .join()
            .await
            .unwrap()
            .status
            .unwrap();

        let (server, stream) = UnixStream::pair().unwrap();
        let _process = runtime.handle_conn(Connection::new(server)).await.unwrap();
        let mut client = Some(vrs::Client::new(Connection::new(stream)));
        // The connection is already open; request_once must never use this path.
        let unused = Path::new("/unused-palette-test.socket");
        let root = request_once(
            &mut client,
            unused,
            protocol::action_request("(:on_click (begin_interaction))").unwrap(),
        )
        .await
        .unwrap()
        .contents
        .unwrap();
        let protocol::Action::PushPage { page: root } = protocol::action(root).unwrap() else {
            panic!()
        };
        assert_eq!(root.get_items, "root_items");
        let response = request_once(
            &mut client,
            unused,
            protocol::query_request(&root.get_items, &root.args, "Read Later").unwrap(),
        )
        .await
        .unwrap()
        .contents
        .unwrap();
        let items = protocol::items(response).unwrap();
        assert_eq!(items[0].title, "Read Later");
        let page = request_once(
            &mut client,
            unused,
            protocol::action_request(&items[0].on_click).unwrap(),
        )
        .await
        .unwrap()
        .contents
        .unwrap();
        let protocol::Action::PushPage { page } = protocol::action(page).unwrap() else {
            panic!()
        };
        assert_eq!(page.debounce_ms, 200);
        let response = request_once(
            &mut client,
            unused,
            protocol::query_request(&page.get_items, &page.args, "").unwrap(),
        )
        .await
        .unwrap()
        .contents
        .unwrap();
        let saved = protocol::items(response).unwrap();
        assert_eq!(saved[0].title, "Saved article");
        let response = request_once(
            &mut client,
            unused,
            protocol::action_request(&saved[0].on_click).unwrap(),
        )
        .await
        .unwrap()
        .contents
        .unwrap();
        assert_eq!(protocol::action(response).unwrap(), protocol::Action::Close);
        let error = request_once(
            &mut client,
            unused,
            protocol::action_request("(:on_click (undefined_command))").unwrap(),
        )
        .await
        .unwrap();
        assert!(error.contents.is_err());
        assert!(
            client.is_some(),
            "a script error is not a broken connection"
        );
        let response = request_once(
            &mut client,
            unused,
            protocol::query_request(&page.get_items, &page.args, "").unwrap(),
        )
        .await
        .unwrap()
        .contents
        .unwrap();
        assert_eq!(protocol::items(response).unwrap(), saved);

        // The user's concrete Save scenario: a captured page fills the argument,
        // so Enter performs the action and closes instead of opening a picker.
        let context = "(((:web/page :title \"Example Domain\" :url \"https://example.test/\")))";
        let response = request_once(
            &mut client,
            unused,
            protocol::query_request("root_items", context, "Save").unwrap(),
        )
        .await
        .unwrap()
        .contents
        .unwrap();
        let actions = protocol::items(response).unwrap();
        assert_eq!(actions[0].title, "Save to Read Later");
        assert_eq!(actions[0].actions[0].title, "Example Domain");
        assert!(!actions
            .iter()
            .any(|item| item.title == "Save a Page to Read Later…"));
        let response = request_once(
            &mut client,
            unused,
            protocol::action_request(&actions[0].on_click).unwrap(),
        )
        .await
        .unwrap()
        .contents
        .unwrap();
        assert_eq!(protocol::action(response).unwrap(), protocol::Action::Close);
        let response = request_once(
            &mut client,
            unused,
            protocol::query_request("root_items", "(())", "searchable.example.test").unwrap(),
        )
        .await
        .unwrap()
        .contents
        .unwrap();
        assert_eq!(
            protocol::items(response).unwrap()[0].title,
            "Project Website"
        );
        let response = request_once(
            &mut client,
            unused,
            protocol::action_request("(:on_click (list :exit 1 :stderr \"Missing app\"))").unwrap(),
        )
        .await
        .unwrap();
        assert!(response.contents.is_err());

        // Exercise each new page through the real protocol, including searches
        // on non-title fields and repeated input without repeating source reads.
        async fn ask(client: &mut Option<vrs::Client>, request: Form) -> Form {
            request_once(client, Path::new("/unused-palette-test.socket"), request)
                .await
                .unwrap()
                .contents
                .unwrap()
        }
        for (entry, callback, query, title) in [
            (
                "Browse Antinote",
                "antinote_items",
                "body-only-needle",
                "Meeting notes",
            ),
            ("Windows", "call_items", "Safari", "Documentation"),
        ] {
            let items = protocol::items(
                ask(
                    &mut client,
                    protocol::query_request("root_items", "(())", entry).unwrap(),
                )
                .await,
            )
            .unwrap();
            assert_eq!(items[0].title, entry);
            let protocol::Action::PushPage { page } = protocol::action(
                ask(
                    &mut client,
                    protocol::action_request(&items[0].on_click).unwrap(),
                )
                .await,
            )
            .unwrap() else {
                panic!()
            };
            assert_eq!(page.get_items, callback);
            assert_eq!(page.debounce_ms, 0);
            for query in ["", query] {
                let results = protocol::items(
                    ask(
                        &mut client,
                        protocol::query_request(&page.get_items, &page.args, query).unwrap(),
                    )
                    .await,
                )
                .unwrap();
                assert_eq!(results[0].title, title);
                if entry == "Browse Antinote" {
                    if query.is_empty() {
                        assert!(results[0].subtitle_spans.is_empty());
                        assert_eq!(results[0].subtitle.as_deref(), Some("2026-09-05"));
                    } else {
                        assert!(results[0]
                            .subtitle_spans
                            .iter()
                            .any(|span| span.matched && span.text == "body-only-needle"));
                        assert_eq!(results[0].aside.as_deref(), Some("2026-09-05"));
                    }
                }
                if ["Browser History", "Browse Antinote", "Windows"].contains(&entry) {
                    assert!(!results[0].actions.is_empty());
                }
                assert_eq!(
                    protocol::action(
                        ask(
                            &mut client,
                            protocol::action_request(&results[0].on_click).unwrap()
                        )
                        .await
                    )
                    .unwrap(),
                    protocol::Action::Close
                );
                if entry == "Windows" {
                    // Cmd-K supplies the window, then the next page fills argument 2.
                    let move_action = results[0]
                        .actions
                        .iter()
                        .find(|action| action.title == "Move Window to Display")
                        .unwrap();
                    let protocol::Action::PushPage { page } = protocol::action(
                        ask(
                            &mut client,
                            protocol::action_request(&move_action.on_click).unwrap(),
                        )
                        .await,
                    )
                    .unwrap() else {
                        panic!()
                    };
                    let displays = protocol::items(
                        ask(
                            &mut client,
                            protocol::query_request(&page.get_items, &page.args, "").unwrap(),
                        )
                        .await,
                    )
                    .unwrap();
                    assert_eq!(displays[0].title, "Display 2");
                    assert_eq!(
                        protocol::action(
                            ask(
                                &mut client,
                                protocol::action_request(&displays[0].on_click).unwrap()
                            )
                            .await
                        )
                        .unwrap(),
                        protocol::Action::Close
                    );
                }
            }
        }
        ask(&mut client, protocol::action_request(
            "(:on_click (if (not? (eq? (list antinote_reads) '(1))) (error \"Repeated source reads\")))"
        ).unwrap()).await;
    }
}
