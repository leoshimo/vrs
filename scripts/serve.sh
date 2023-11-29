#!/usr/bin/env sh
# serve.sh - Trivial Serve

while true; do
    PID=$(RUST_LOG=debug cargo run --bin vrsd > vrsd.log) &
    sleep 1
    ./scripts/launcher.ll &
    tail -f vrsd.log
    wait $PID
done
