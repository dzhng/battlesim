# Native client

The native client on `wgpu` ([slice 25](../../specs/native-rendering-parity/slices/25-native-on-wgpu.md)).
It runs the browser's own shaders and passes over the [presentation crate](../../crates/presentation/README.md)'s
`World` and `Frame`, and decides nothing per entity itself.

- **Shaders and pipelines** are the browser's [export](../../crates/pipelines/README.md),
  built from `pipelines.json` at start. The export is read strictly: a setting
  the client does not know fails the build rather than being dropped, and any
  pipeline the device refuses fails the start, listing each one.
- **The frame** is one ordered list of passes, encoded in order each drawn
  frame, as Bevy draws since 0.19 and as the browser's `battleFrame` does. It
  has no render graph. A pass reads the frame's `View` and the sized targets.
- **Uniforms** are records the presentation crate resolves (the sky's light,
  the sun, the haze, the bloom, the grade, the view's sky rays). The client
  uploads their bytes as they are, and `crates/pipelines` holds each record to
  the struct its shader declares. A pass binds groups by the layout they fill,
  never by number.
- **Models** are the browser's appearance bundles from `assets/runtime` (Git
  LFS; pull the ones a scene needs), read as the browser reads them: the
  catalog, gzip transport, the bundle container and its texture files. Every
  `World.appearances` row is loaded once into the browser's vertex layout, and
  each `Frame` batch is one run of instances over one row's tier. A corpse is
  its body posed once by the presentation crate's rest palette; so is a body
  at the impostor tier (its far pose), since impostor cards are not baked yet.
- **The ground** is the presentation crate's, resolved from the saved map
  by the simulation's own world code (`static_world::files::ground`): the
  terrain's tables, its meshes (terrain, skirt, backdrop, water), the grass's
  tables and blade shapes, and the side's learned scars. The client uploads
  them and draws the terrain into the depth prepass, the cascades and the lit
  world. The grass grows on the GPU over the window the crate fits to each
  view, through the exported build, finish and indirect draw.
- **Sun shadows** are four cascades the presentation crate fits to each
  frame's camera over the map's box (`presentation::shadow`): the receiver
  block and each cascade's caster camera are records the client uploads. The
  ground, the models and the buildings cast into them, a unit one tier
  coarser than it draws and never as a card.
- **Buildings** are the frame's static batches: the map's buildings placed
  from the template art, in the state the side knows. A replay frames only the
  plates it draws, so it asks for every static batch on those frames.
- **Effects** are the frame's effect instances (`Presentation::effects`),
  drawn into the resolved lit world over the browser's own flipbook sheets
  (`assets/third-party/effects`, Git LFS), and the cast lights the
  presentation chose (`Presentation::lights`) light every lit material and
  the smoke.
- **Fog's mask** runs as the browser's does: the world resolves its fog mask
  beside the lit world, and the mask passes compose the fogged world into
  post's input and draw the rim after post, in the fixture's unseen look
  (`presentation::fog` resolves it).
- **Sound** is the presentation crate's: it decides every voice, and the
  client's `audio` module carries the commands out on `kira` (a track per
  voice under its bus, low-pass, equal-power pan, reverb sends, a limiter),
  with the recordings in `assets/runtime/audio/clips` (Git LFS) and the
  synthesized sounds composed by the same Rust bank the browser's decisions
  were made against. `--sound <wav>` renders a replay's sound to a file; it is
  never played aloud by the replay, and the tests render to a buffer.
- **What is bound but not drawn yet** reads as absent in every shader: fog
  of war's sight (every surface seen) and the painted ground marks (the UI
  paints them; a replay has no UI).

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
