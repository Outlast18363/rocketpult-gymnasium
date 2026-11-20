#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
CARGO_BIN="${CARGO:-$HOME/.cargo/bin/cargo}"
"$CARGO_BIN" build --release --features play --bin rocketpult --target wasm32-unknown-unknown
cp target/wasm32-unknown-unknown/release/rocketpult.wasm web/rocketpult.wasm
printf '\nWeb build ready in web/. Start with: python3 scripts/serve.py\n'
