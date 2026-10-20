#!/bin/sh
set -eu
SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
export CARGO_HOME="${CARGO_HOME:-/mnt/d/ChatGPT/hwp/.rust-tools/cargo}"
export RUSTUP_HOME="${RUSTUP_HOME:-/mnt/d/ChatGPT/hwp/.rust-tools/rustup}"
export PATH="$CARGO_HOME/bin:$PATH"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$SCRIPT_DIR/target}"
cd "$SCRIPT_DIR"
exec cargo "$@"
