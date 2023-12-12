#!/usr/bin/env sh
# serve.sh - Trivial Serve

if [ "$TMUX" ]; then
    tmux rename-window "serve"
fi

while true; do
    PID=$(RUST_LOG=debug cargo run --bin vrsd > vrsd.log) &
    sleep 1
    ./scripts/launcher.ll &
    wait $PID
done
