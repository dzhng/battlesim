#!/usr/bin/env bash
# Vercel's install step: the build machines have Node but no Rust, and the
# build compiles the simulation to WebAssembly first (`bun run build:wasm`).
set -euo pipefail
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --target wasm32-unknown-unknown
. "$HOME/.cargo/env"
curl -sSf https://rustwasm.github.io/wasm-pack/installer/init.sh | sh
npm install -g bun
bun install --cwd web
