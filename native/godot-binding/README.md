# Godot binding

This crate is the native client's GDExtension. Two parts:

- **The native core** the Godot project loads: so far the static layers'
  per-frame walk (`src/static_layers`), and the buildings' walk shared with
  the browser (`src/building_walk.rs` over `crates/presentation`'s `static_world`), described in
  [the project's readme](../godot/README.md#the-rust-extension).
  `native/godot/binding/build.sh` builds and places it.
- **The binding smoke** (`src/lib.rs`'s node and version probe), the first
  live seam for the simulation, while the binding architecture is measured.
  The Rust simulation remains in `sim`; later passes add commands, fixed
  ticks, bulk observations, replay and digest only after the offline probe
  and lifecycle checks agree.

Build the smoke's macOS extension with Godot 4.7's API:

```text
cargo build -p godot-binding --release
```

The descriptor in [`native/godot/binding_smoke/`](../godot/binding_smoke/)
targets the arm64 Mac library and Godot 4.7's GDExtension API. The library is
build output and is never committed.

The smoke runs the offline `native_binding_probe` and this extension on the
same scenario, inside a scratch Godot project under `throwaway/`, and requires
the same digest, publication and replay from both. It needs the debug
`godot-binding` library and the `native_binding_probe` example built in
`CARGO_TARGET_DIR`:

```text
cargo build -p godot-binding
cargo build -p sim --example native_binding_probe
native/godot/binding_smoke/binding_smoke.sh <godot> <scenario.json>
```

`native_binding_probe --write-fixture <path>` writes a small scenario for it.
