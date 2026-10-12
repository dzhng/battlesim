# Native client

The native client on `wgpu` ([slice 25](../../specs/native-rendering-parity/slices/25-native-on-wgpu.md)),
which replaces [Godot](../godot/README.md). It runs the browser's own shaders
and passes over the [presentation crate](../../crates/presentation/README.md)'s
`World` and `Frame`, and decides nothing per entity itself.

- **Shaders and pipelines** are the browser's [export](../../crates/pipelines/README.md),
  built from `pipelines.json` at start. The export is read strictly: a setting
  the client does not know fails the build rather than being dropped, and any
  pipeline the device refuses fails the start, listing each one.
- **The frame** is one ordered list of passes, encoded in order each drawn
  frame, as Bevy draws since 0.19 and as the browser's `battleFrame` does. It
  has no render graph. A pass reads the frame's `View` and the sized targets.
- **Uniforms** are records the presentation crate resolves (the sky's light,
  the bloom, the grade, the view's sky rays). The client uploads their bytes as
  they are, and `crates/pipelines` holds each record to the struct its shader
  declares.

## Running a parity scene

The binary replays one scene of a browser parity run (the control the
[spec](../../specs/native-rendering-parity/README.md) names): it loads the
world, frames every record through the presentation crate, and writes each
plate the browser wrote, under the same name. Its usage text lists the flags.
Run it under the GPU lock.

The window opens borderless and off-screen by default, so nothing appears on
the user's screen. On macOS, `wgpu` presents nothing to a window that isn't
visible (off-screen, minimised or hidden all report `Occluded`), so those runs
draw unpaced; plates never depend on presenting.

## Checks

`tests/pipelines.rs` builds every exported pipeline on this machine's device
(it needs the GPU lock). Plates are judged against the control's, region by
region, with the spec's screenshot skills.
