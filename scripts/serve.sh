#!/usr/bin/env sh
# serve.sh - Trivial Serve
#
# For Dev:
#     ./serve.sh dev
#
# For Live-On:
#     ./serve.sh

MODE="live-on"
if [ "$1" = "dev" ]; then
    MODE="dev"
fi

if [ "$TMUX" ]; then
    tmux rename-window "vrs-srv-$MODE"
fi

CARGO_ARGS=
if [ "$MODE" = "live-on" ]; then
    CARGO_ARGS="--release"
fi

echo "Mode: $MODE"

while true; do
     PID=$(RUST_LOG=debug cargo run --bin vrsd "$CARGO_ARGS" > "vrsd-$MODE.log") &

     while true; do
         cargo run --bin vrsctl $CARGO_ARGS -- --command ':healthcheck' >/dev/null 2>&1
         if [ $? -eq 0 ]; then
             cargo run --bin vrsctl $CARGO_ARGS ./scripts/launcher.ll >/dev/null
             cargo run --bin vrsctl $CARGO_ARGS ./scripts/chat.ll >/dev/null
             cargo run --bin vrsctl $CARGO_ARGS ./scripts/system_appearance.ll >/dev/null
             cargo run --bin vrsctl $CARGO_ARGS ./scripts/os_notify.ll >/dev/null
             cargo run --bin vrsctl $CARGO_ARGS ./scripts/os_browser.ll >/dev/null
             cargo run --bin vrsctl $CARGO_ARGS ./scripts/vrsjmp.ll >/dev/null

             # Restart vrsjmp if live-on
             if [ "$MODE" = "live-on" ]; then
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
