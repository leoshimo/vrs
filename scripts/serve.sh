#!/usr/bin/env sh
# Start the runtime and, unless running headless, the GUI. Service
# configuration lives in ../init.ll.
#
# Usage:
#   ./scripts/serve.sh          # release runtime and GUI
#   ./scripts/serve.sh dev      # debug runtime and GUI
#   ./scripts/serve.sh headless # release runtime only

set -u

MODE="${1:-live-on}"
CARGO_ARGS=""
if [ "$MODE" != "dev" ]; then
    CARGO_ARGS="--release"
fi

if [ "${TMUX:-}" ]; then
    tmux rename-window "vrs-srv-$MODE"
fi

cargo run $CARGO_ARGS --bin vrsd -- --init ./init.ll > "vrsd-$MODE.log" 2>&1 &
VRSD_PID=$!
VRSJMP_PID=""

cleanup() {
    kill "$VRSD_PID" 2>/dev/null || true
    if [ -n "$VRSJMP_PID" ]; then
        kill "$VRSJMP_PID" 2>/dev/null || true
    fi
}
trap cleanup INT TERM EXIT

# vrsd binds its client socket only after init.ll completes. Do not launch the
# GUI until the daemon is accepting requests, and stop if initialization fails.
until cargo run $CARGO_ARGS --bin vrsctl -- --command ':healthcheck' >/dev/null 2>&1; do
    if ! kill -0 "$VRSD_PID" 2>/dev/null; then
        wait "$VRSD_PID"
        VRSD_STATUS=$?
        echo "vrsd exited before initialization completed (status $VRSD_STATUS); see vrsd-$MODE.log" >&2
        if [ "$VRSD_STATUS" -eq 0 ]; then
            VRSD_STATUS=1
        fi
        exit "$VRSD_STATUS"
    fi
    sleep 1
done

if [ "$MODE" != "headless" ]; then
    cargo run $CARGO_ARGS --bin vrsjmp &
    VRSJMP_PID=$!
    wait "$VRSD_PID" "$VRSJMP_PID"
else
    wait "$VRSD_PID"
fi
