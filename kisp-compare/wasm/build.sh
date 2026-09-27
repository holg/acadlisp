#!/usr/bin/env bash
# Build the wasm32 benchmark module. Needs: rustup target add wasm32-unknown-unknown,
# cargo install wasm-bindgen-cli (the version pinned in Cargo.lock), node.
set -euo pipefail
cd "$(dirname "$0")"
cargo build --release --target wasm32-unknown-unknown
WASM="${CARGO_TARGET_DIR:-target}/wasm32-unknown-unknown/release/kisp_compare_wasm.wasm"
wasm-bindgen --target nodejs --out-dir pkg "$WASM"
echo "ok: pkg/  ->  node bench.mjs [runs] [filter]"
