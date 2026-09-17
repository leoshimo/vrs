# vrsjmp

vrsjmp is a desktop client for VRS.

```sh
cd vrsjmp
pnpm install
pnpm tauri dev
```

Use macOS 14+, Node 22.13+, and Rust. The debug shortcut is ⌃⌘⇧Space; release uses ⌘Space.
`pnpm tauri build` creates the native app. `scripts/serve.sh` builds the frontend
before starting the runtime and GUI.

Theme configuration persists in `~/.vrsjmp-ui.ll` and updates connected clients:

```lisp
(bind_srv :vrsjmp)
(set_ui_config '(:theme :warm :appearance :dark))
```

Themes: `:neutral`, `:warm`, `:cool`. Appearance: `:system`, `:light`, `:dark`.
