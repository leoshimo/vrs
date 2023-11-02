#!/usr/bin/env sh
# serve.sh - Trivial Serve

while true; do
    PID=$(RUST_LOG=debug cargo run --bin vrsd) &
    sleep 1
    ./scripts/launcher.ll &
    wait $PID
done
