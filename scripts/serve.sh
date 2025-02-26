#!/usr/bin/env sh
# serve.sh - Trivial Serve
#
# Examples
#     ./serve.sh       - Live-On
#     ./serve.sh dev   - Development
#     ./serve.sh demo  - Demo
#

MODE="$1"

if [ -z "$MODE" ]; then
    MODE="live-on"
fi


if [ "$TMUX" ]; then
    tmux rename-window "vrs-srv-$MODE"
fi

CARGO_ARGS=
if [ "$MODE" != "dev" ]; then
    CARGO_ARGS="--release"
fi

echo "Mode: $MODE"

while true; do
     PID=$(cargo run --bin vrsd "$CARGO_ARGS" > "vrsd-$MODE.log") &

     while true; do
         cargo run --bin vrsctl $CARGO_ARGS -- --command ':healthcheck' >/dev/null 2>&1
         if [ $? -eq 0 ]; then
             cargo run --bin vrsctl $CARGO_ARGS ./scripts/chat.ll >/dev/null
             cargo run --bin vrsctl $CARGO_ARGS ./scripts/system_appearance.ll >/dev/null
             cargo run --bin vrsctl $CARGO_ARGS ./scripts/todos.ll >/dev/null
             cargo run --bin vrsctl $CARGO_ARGS ./scripts/cmd_macro.ll >/dev/null

             cargo run --bin vrsctl $CARGO_ARGS ./scripts/interfacegen.ll >/dev/null

             cargo run --bin vrsctl $CARGO_ARGS ./scripts/os_maps.ll >/dev/null
             cargo run --bin vrsctl $CARGO_ARGS ./scripts/os_notes.ll >/dev/null
             cargo run --bin vrsctl $CARGO_ARGS ./scripts/obsidian.ll >/dev/null

             cargo run --bin vrsctl $CARGO_ARGS ./scripts/os_display.ll >/dev/null
             cargo run --bin vrsctl $CARGO_ARGS ./scripts/os_window.ll >/dev/null
             cargo run --bin vrsctl $CARGO_ARGS ./scripts/os_notify.ll >/dev/null
             cargo run --bin vrsctl $CARGO_ARGS ./scripts/os_browser.ll >/dev/null
             cargo run --bin vrsctl $CARGO_ARGS ./scripts/os_screencap.ll >/dev/null
             cargo run --bin vrsctl $CARGO_ARGS ./scripts/os_cal.ll >/dev/null
             cargo run --bin vrsctl $CARGO_ARGS ./scripts/os_clipboard.ll >/dev/null

             cargo run --bin vrsctl $CARGO_ARGS ./scripts/github.ll >/dev/null
             cargo run --bin vrsctl $CARGO_ARGS ./scripts/safari_history.ll >/dev/null
             cargo run --bin vrsctl $CARGO_ARGS ./scripts/youtube.ll >/dev/null

             if [ "$MODE" = "demo" ]; then
                 cargo run --bin vrsctl $CARGO_ARGS ./scripts/rlist_demo.ll >/dev/null
                 cargo run --bin vrsctl $CARGO_ARGS ./scripts/nl_shell_demo.ll >/dev/null

                 sleep 3
                 cargo run --bin vrsctl $CARGO_ARGS -- --command "(begin (bind_srv :rlist) (add_rlist \"File over app\" \"https://stephango.com/file-over-app\"))"
                 cargo run --bin vrsctl $CARGO_ARGS -- --command '(exec "osascript" "-e" "tell application id \"tracesOf.Uebersicht\" to refresh widget id \"vrs_shell-jsx\"")'
             else
                 cargo run --bin vrsctl $CARGO_ARGS ./scripts/rlist.ll >/dev/null
                 cargo run --bin vrsctl $CARGO_ARGS ./scripts/nl_shell.ll >/dev/null
             fi

             cargo run --bin vrsctl $CARGO_ARGS ./scripts/nl_scheduler.ll >/dev/null

             cargo run --bin vrsctl $CARGO_ARGS ./scripts/vrs_shell_refresh.ll >/dev/null

             if [ "$MODE" = "demo" ]; then
                 cargo run --bin vrsctl $CARGO_ARGS ./scripts/vrsjmp_demo.ll >/dev/null
             else
                 cargo run --bin vrsctl $CARGO_ARGS ./scripts/vrsjmp.ll >/dev/null
             fi

             # Restart vrsjmp if not dev
             if [ "$MODE" != "dev" ]; then
                 pkill -ax "vrsjmp"
                 PID=$(RUST_LOG=debug cargo run --bin vrsjmp "$CARGO_ARGS" > /dev/null) &
                 wait $PID
             fi

             echo "Launched Services"
             break
         fi
         sleep 2
     done
     wait $PID
done
