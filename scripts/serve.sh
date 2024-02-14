#!/usr/bin/env sh
# serve.sh - Trivial Serve

if [ "$TMUX" ]; then
    tmux rename-window "serve"
fi

while true; do
    PID=$(RUST_LOG=debug cargo run --bin vrsd --release > vrsd.log) &

    while true; do
        ./scripts/launcher.ll >/dev/null 2>&1
        if [ $? -eq 0 ]; then
            echo "Successfully started launcher.ll"
            break
        fi
        sleep 2
    done
    wait $PID
done
