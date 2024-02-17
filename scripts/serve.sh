#!/usr/bin/env sh
# serve.sh - Trivial Serve
#
# For Dev:
#     ./serve.sh
# For Live-On Dev:
#     ./serve.sh --release

if [ "$TMUX" ]; then
    tmux rename-window "serve"
fi

while true; do
    PID=$(RUST_LOG=debug cargo run --bin vrsd $@ > vrsd.log) &

    while true; do
        cargo run --bin vrsctl $@ ./scripts/launcher.ll >/dev/null 2>&1
        if [ $? -eq 0 ]; then
            cargo run --bin vrsctl $@ ./scripts/chat.ll >/dev/null 2>&1
            cargo run --bin vrsctl $@ ./scripts/system_appearance.ll >/dev/null 2>&1
            echo "Launched Services"
            break
        fi
        sleep 2
    done
    wait $PID
done
