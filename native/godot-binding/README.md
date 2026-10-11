# Godot binding smoke

This crate is the first live GDExtension seam. It intentionally exposes only
an extension-owned `Node` and version probe while the binding architecture is
measured. The Rust simulation remains in `sim`; later passes add commands,
fixed ticks, bulk observations, replay and digest only after the offline probe
and lifecycle checks agree.

Build a macOS extension with Godot 4.7's API:

```text
cargo build -p godot-binding --release
```

The checked-in descriptor and `native/godot-spike/binding_smoke.tscn` target the Godot 4.2+
GDExtension ABI and the arm64 Mac library. Copy the built
`target/release/libgodot_binding.dylib` into the project's
`addons/battle_binding/` directory before attaching it to a Godot project. The
generated library is intentionally not committed; the target checkout owns its
build output.

The live smoke command is:

```text
cargo build -p godot-binding --release
cp target/release/libgodot_binding.dylib native/godot-spike/addons/battle_binding/
Godot --headless --path native/godot-spike --scene res://binding_smoke.tscn --quit-after 60 --no-header
```

