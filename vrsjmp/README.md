# vrsjmp

vrsjmp is a desktop client for VRS.

```sh
cd vrsjmp
pnpm install
pnpm tauri dev
```

Use macOS 14+, Node 22.13+, and Rust. The debug shortcut is ⌃⌘⇧Space; release uses ⌘Space.
`pnpm tauri build --features custom-protocol` creates the native app with its frontend embedded. From the repository root,
`./scripts/serve.sh dev` starts the debug runtime and GUI with hot reload.
`./scripts/serve.sh` builds and runs the release runtime and GUI.

For a native UI preview, pass `--preview` to the app, optionally with `--socket PATH`
to use a test runtime. The palette stays open without taking keyboard focus or
registering a global shortcut. Use its mouse or accessibility controls to test it;
action completion still dismisses it.

Theme configuration persists in `~/.vrsjmp-ui.ll` and updates connected clients:

```lisp
(bind_srv :vrsjmp)
(set_ui_config '(:theme :warm))
```

Themes: `:neutral`, `:warm`, `:cool`. Light and dark follow the system setting.
