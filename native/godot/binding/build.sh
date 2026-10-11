#!/bin/sh
# Builds the Rust GDExtension (native/godot-binding) and puts its library
# where binding/battle_binding.gdextension loads it. The library is build
# output, never committed. tests/run.sh and release.sh run this; run it
# before an editor run whenever the crate changed. CARGO_TARGET_DIR defaults to the
# checkout's throwaway/target.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
repo=$(cd "$here/../../.." && pwd)
target=${CARGO_TARGET_DIR:-$repo/throwaway/target}
CARGO_TARGET_DIR="$target" cargo build --manifest-path "$repo/Cargo.toml" -p godot-binding --release
cp "$target/release/libgodot_binding.dylib" "$here/libgodot_binding.dylib"
