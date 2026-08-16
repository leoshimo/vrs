#!/usr/bin/env sh
# Start the runtime and GUI. Service configuration lives in ../init.ll.

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

# vrsjmp already reports a useful connection error if daemon initialization
# takes longer than this; keep orchestration here intentionally lightweight.
sleep 1
cargo run $CARGO_ARGS --bin vrsjmp &
VRSJMP_PID=$!

wait "$VRSD_PID" "$VRSJMP_PID"
