# Slice 02 — shared native/browser core and migration verdict

**Status:** normative target 2026-10-27; implementation gates remain open.

The native model/material/static-batch and scenery implementations now live in
`renderer-core` and execute in both shells. The browser's parallel asset/state/
vertex admission and nonindexed model passes have been removed. Production
camera comparisons must install BattleFrame's static frame as well as its
dynamic frame; both use the complete production appearance generation. The
captured run now proves transport and recovery, while missing sky and different
lighting/fog/ground keep visual acceptance red. Current measurements and the
next discriminating experiment live in [research](../research.md#current-task--native-owner-replacement-2026-10-28).

The spike no longer treats these as equal final choices. The clean architecture
is:

1. **One Rust renderer runtime:** every GPU resource, pass, material, model,
   lifecycle and frame boundary is owned once by Rust `wgpu`.
2. **Two platform shells:** native and browser own only surface/device setup,
   asset transport, window/DOM integration and audio/platform diagnostics.
3. **TypeGPU deletion:** TypeGPU is temporary shader authoring/control input and
   is deleted after migration unless a separate build-only authoring spike proves
   its value against its maintenance cost.

Do not ship the bounded graph. It proves browser `wgpu` feasibility, but it is a
separately written renderer. The native client already owns most of the desired
Rust pass graph; the implementation must move those owners into one runtime and
delete the duplicate browser graph after the gates pass.

The current bounded production-shaped run is useful only as a transport
diagnostic: the latest interleaved browser BattleFrame measures
`154.0 / 32.8 / 140.0 ms` and the standalone Rust graph `8.9 / 21.0 / 30.4 ms`,
with image distance `47.03`.
Those numbers are not a migration verdict because the Rust row does not reuse
the native renderer implementation and still omits material, shadow sampling,
fog visibility, overlays, post treatment and lifecycle parity.

## Ownership gap and deletion plan

| Contract | Browser owner | Shared-core target | Evidence |
| --- | --- | --- | --- |
| Canvas, adapter, device and surface | BattleFrame caller and TypeGPU root | Thin native/browser platform shells | Same WebGPU canvas seam works |
| Terrain, models and presentation records | `worldPass` / `modelLayer` | `native/client/src/{ground,models,scenery,statics}.rs` | Existing native client already consumes the records |
| Shadows, fog, effects, post and overlays | BattleFrame pass owners | Shared Rust runtime | Shadow resources shared; remaining pass owners open |
| Shader and pipeline metadata | TypeGPU export | `crates/pipelines` | Already shared with native |
| Resource registry, generation, rebuild and device loss | `GpuRegistry` / `SizedTargets` | Shared Rust core with platform-owned surface | Must be measured after extraction |

The production frame's pass order is shadows → depth → fog compute → world →
effects → fog mask/rim → post → overlays/composite → high-air plane. The next
slice must run that order from one Rust runtime in both targets. Platform shells
may own only surface/device setup, runtime file loading and audio; they must not
fork pass logic or resource ownership.

The latest bounded run records `154.0 / 32.8 / 140.0 ms` for production
BattleFrame setup / CPU encode / queue-drain boundary and `8.9 / 21.0 / 30.4 ms`
for the standalone Rust graph. Queue drain is a submission/compositor boundary,
not a GPU timestamp. Keep these as transport diagnostics; the migration decision
waits for the shared native/browser core comparison.

The final migration gate must compare:

- shared Rust core LOC and GPU-construction counts against the current TypeGPU
  renderer, including the platform-shell delta;
- native and browser screenshots from the same frozen presentation frame;
- startup, CPU encode, GPU timestamps, queue submission, memory, resize,
  rebuild and device-loss recovery;
- whether deleting TypeGPU and the duplicate browser graph leaves one clear GPU
  owner with lower total complexity, even if frame time is equal or slightly
  worse.

## Required deletion gates

The migration is not complete when the Rust row merely renders. It is complete
when these deletions are safe:

- browser TypeGPU device, pass, target and resource owners are gone;
- standalone production-shaped Rust code is gone;
- native and browser call one Rust prepare/encode/submit boundary;
- all material, shadow, fog, effect, post and overlay features are represented
  by the shared runtime or explicitly removed from the product;
- resize, rebuild, surface loss and device loss return both shells to baseline;
- matched native/browser screenshots and GPU/CPU timings are recorded;
- the final source accounting shows the deleted owners, not just the added core.

## Required implementation seam

Extract a reusable Rust renderer crate from `native/client/src/` with three
layers:

1. **Shared core:** frame inputs, pass ordering, target allocation, pipeline
   loading, buffers/textures/bind groups, shadows, world/models, scenery,
   effects, fog, post, overlays, resize and reset.
2. **Native shell:** `winit` surface, filesystem runtime assets, desktop
   adapter/device policy, readback and audio.
3. **Browser shell:** `wasm-bindgen`/`BrowserWebGpu` surface, fetched runtime
   assets, browser adapter/device policy and JavaScript frame bridge.

The shared core must not import `winit`, `std::fs`, desktop audio, or browser
DOM APIs. Both shells must call the same core frame entry point with the same
`presentation` records and exported pipeline metadata. A browser-only copy of
`frame.rs`, `world.rs`, `models.rs`, `fog.rs`, `post.rs` or `effects.rs` fails
the slice even if its pixels look correct.

The shared-core checkpoints now cover resources, pass-list ordering, pipeline
construction, depth prepass, and world/material sequencing. `renderer-core`
owns target formats, attachments, reverse-Z depth policy, sky/world ordering,
layer/surface iteration and source-provider pipeline construction; native adapts
its concrete ground, scenery, fog, models and sky through explicit traits.
`cargo check -p native-client --target wasm32-unknown-unknown` passes, as do
`presentation` and `presentation-wasm`. A WASM `Library::from_bundle` API now
accepts fetched BGAT and texture bytes and parses them into shared asset records.
The browser shell exercises that API, uploads the first authored albedo
texture, and draws the first indexed mesh through the shared model owner. The
latest asset proof consumed 3,960,005 compressed wire bytes,
4,453,752 decoded bundle bytes and 9,437,706 decoded texture bytes, producing
27 textures, 12 materials, 98,021 vertices and 236,706 indices; the shared draw
selects a 37,002-vertex indexed tier. This is still not a production screenshot
comparison: native material rows, fog, shadows, effects, post and overlays
remain outside the shared browser fixture.
