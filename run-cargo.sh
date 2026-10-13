#!/bin/sh
set -eu
export CARGO_HOME=/mnt/d/ChatGPT/hwp/.rust-tools/cargo
export RUSTUP_HOME=/mnt/d/ChatGPT/hwp/.rust-tools/rustup
export PATH="$CARGO_HOME/bin:$PATH"
export CARGO_TARGET_DIR=/mnt/d/ChatGPT/hwp/hwp-engine/target
cd /mnt/d/ChatGPT/hwp/hwp-engine
exec cargo "$@"
