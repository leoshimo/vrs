#!/usr/bin/env sh
# install.sh - Install All Binaries

set -eu
SCRIPT_ROOT=$(CDPATH= cd "$(dirname "$0")/.." && pwd)
cd "$SCRIPT_ROOT"

pnpm --dir vrsjmp install --frozen-lockfile
pnpm --dir vrsjmp build

cargo install --locked --path vrsctl
cargo install --locked --path vrsd
cargo install --locked --path vrsjmp/src-tauri --features custom-protocol
