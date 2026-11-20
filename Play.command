#!/bin/sh
set -eu
cd "$(dirname "$0")"
if [ -x target/release/rocketpult ]; then
  exec target/release/rocketpult
fi
exec "$HOME/.cargo/bin/cargo" run --release --features play --bin rocketpult
