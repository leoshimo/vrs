# vrsjmp

vrsjmp is a desktop client for VRS.

```sh
cd vrsjmp
pnpm install
pnpm tauri dev
```

Use macOS 14+, Node 22.13+, and Rust. The debug shortcut is ⌃⌘⇧Space; release uses ⌘Space.
`pnpm tauri build` creates the native app. From the repository root,
`./scripts/serve.sh dev` starts the debug runtime and GUI with hot reload.
`./scripts/serve.sh` builds and runs the release runtime and GUI.

Theme configuration persists in `~/.vrsjmp-ui.ll` and updates connected clients:

```lisp
(bind_srv :vrsjmp)
(set_ui_config '(:theme :warm))
```

Themes: `:neutral`, `:warm`, `:cool`. Light and dark follow the system setting.
