#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
cargo build --release --locked
# Keep a stable executable path for the native Render start command.
mkdir -p bin
cp target/release/biq-backend bin/biq-backend
