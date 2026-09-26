//! A command palette takes keyboard focus without activating its application.

use anyhow::{Context, Result};
use tauri::WebviewWindow;
use tauri_nspanel::{
    objc2::MainThreadMarker,
    objc2_app_kit::{NSScreen, NSWindow, NSWindowCollectionBehavior, NSWindowStyleMask},
    ManagerExt, PanelLevel, WebviewWindowExt,
};

mod native {
    tauri_nspanel::tauri_panel! {
        panel!(PalettePanel {
            config: {
                can_become_key_window: true,
                can_become_main_window: false,
                is_floating_panel: true,
                hides_on_deactivate: false
            }
        })
    }
}
use native::PalettePanel;

pub fn configure(window: &WebviewWindow, preview: bool) -> Result<()> {
    MainThreadMarker::new().context("Palette setup must run on the main thread")?;
    if preview {
        let native_window = window.ns_window()?;
        // SAFETY: Tauri owns this live NSWindow; setup borrows it on the main thread.
        let native_window = unsafe { native_window.cast::<NSWindow>().as_ref() }
            .context("Palette has no native macOS window")?;
        native_window.setCollectionBehavior(
            native_window.collectionBehavior() | NSWindowCollectionBehavior::MoveToActiveSpace,
        );
        return Ok(());
    }
    let panel = window.to_panel::<PalettePanel>()?;
    panel.add_style_mask(NSWindowStyleMask::NonactivatingPanel)?;
    panel.set_level(PanelLevel::Floating.value());
    // A nonactivating panel must be eligible for the current Space without
    // activating its app to move there. The renderer dismisses it on blur.
    panel.set_collection_behavior(
        NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::CanJoinAllApplications
            | NSWindowCollectionBehavior::FullScreenAuxiliary,
    );
    Ok(())
}

pub async fn show(window: WebviewWindow) -> Result<()> {
    on_main_thread(window, |window, main_thread| {
        let panel = window
            .get_webview_panel("main")
            .map_err(|_| anyhow::anyhow!("Palette panel is unavailable"))?;
        if !panel.is_visible() {
            // Choose the screen receiving keyboard input before the palette
            // takes focus, rather than always moving to the primary monitor.
            if let Some(screen) = NSScreen::mainScreen(main_thread) {
                let screen = screen.frame();
                let mut frame = panel.as_panel().frame();
                frame.origin.x =
                    screen.origin.x + (screen.size.width - frame.size.width).max(0.) / 2.;
                frame.origin.y =
                    screen.origin.y + (screen.size.height - frame.size.height).max(0.) / 2.;
                panel.as_panel().setFrameOrigin(frame.origin);
            }
        }
        // Tauri's set_focus() also activates NSApplication, which can leave a
        // full-screen Space. Only make this nonactivating panel key instead.
        panel.show_and_make_key();
        Ok(())
    })
    .await
}

pub async fn hide(window: WebviewWindow) -> Result<()> {
    on_main_thread(window, |window, _| {
        let panel = window
            .get_webview_panel("main")
            .map_err(|_| anyhow::anyhow!("Palette panel is unavailable"))?;
        // Ordering out releases keyboard focus back to the underlying app.
        // Do not hide or activate NSApplication as part of palette dismissal.
        panel.hide();
        Ok(())
    })
    .await
}

async fn on_main_thread(
    window: WebviewWindow,
    action: impl FnOnce(WebviewWindow, MainThreadMarker) -> Result<()> + Send + 'static,
) -> Result<()> {
    let (reply, response) = tokio::sync::oneshot::channel();
    let target = window.clone();
    window.run_on_main_thread(move || {
        let result = MainThreadMarker::new()
            .context("Palette window operations must run on the main thread")
            .and_then(|main_thread| action(target, main_thread));
        let _ = reply.send(result);
    })?;
    response
        .await
        .context("Palette window operation was cancelled")?
}
