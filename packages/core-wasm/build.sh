#!/usr/bin/env bash
# Build @openhfp/core-wasm: compile the hfp-core read side to wasm32-unknown-unknown and
# run wasm-bindgen (target web) into dist/. Requires the wasm32-unknown-unknown target and
# wasm-bindgen-cli (pin the same version as the wasm-bindgen crate, currently 0.2.x).
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$HERE/../.."

cargo build -p hfp-wasm --target wasm32-unknown-unknown --release --manifest-path "$ROOT/Cargo.toml"
wasm-bindgen --target web --out-dir "$HERE/dist" \
  "$ROOT/target/wasm32-unknown-unknown/release/hfp_wasm.wasm"
echo "core-wasm built -> $HERE/dist"
