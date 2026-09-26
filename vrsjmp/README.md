# vrsjmp

vrsjmp is a desktop client for VRS.

```sh
cd vrsjmp
pnpm install
pnpm tauri dev
```

Use macOS 14+, Node 22.13+, and Rust. The debug shortcut is ⌃⌘⇧Space; release uses ⌘Space.
The macOS palette appears over the current Space, including full-screen apps,
and takes keyboard input without activating vrsjmp. Escape or moving focus to
another window dismisses it, leaving the underlying app in place.

`pnpm tauri build --features custom-protocol` creates the native app with its frontend embedded. From the repository root,
`./scripts/serve.sh dev` starts the debug runtime and GUI with hot reload.
`./scripts/serve.sh` builds and runs the release runtime and GUI.

For a native UI preview, pass `--preview` to the app, optionally with `--socket PATH`
to use a test runtime. The palette stays open without taking keyboard focus or
registering a global shortcut. Use its mouse or accessibility controls to test it;
action completion still dismisses it.

To check native window behavior, leave vrsjmp running and hidden, switch to a
full-screen app, and invoke the shortcut. Type a query, dismiss with Escape,
reopen, and move focus to another window. Neither opening nor dismissal should
switch Spaces. Repeat on an ordinary desktop; `--preview` should stay open when
another window receives focus.

Theme configuration persists in `~/.vrsjmp-ui.ll` and updates connected clients:

```lisp
(bind_srv :vrsjmp)
(set_ui_config '(:theme :warm))
```

Themes: `:neutral`, `:warm`, `:cool`. Light and dark follow the system setting.
