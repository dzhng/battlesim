# Renderer cleanup

**Status:** done, 2026-10-30.

The shared Rust renderer ([`crates/renderer-core`](../../../crates/renderer-core/README.md),
closed spec [`one-renderer`](../one-renderer/README.md)) shipped with
structural debt: shader code that didn't live with its concept, GPU buffer
layouts packed by hand and checked by nothing, and four concepts with two
owners. This cleanup gave each concept one owner, and proved every structural
change by leaving the native client's parity plates byte-identical. The
decisions the build made on its own are in [the choices ledger](choices.md).

## What must stay true

**GPU record layouts have one owner, and a test reads them.** Every struct a
shader reads through a uniform or storage binding is a `records!` record: in
[`presentation::records`](../../../crates/presentation/src/records.rs) when
presentation writes it, in [`renderer_core::records`](../../../crates/renderer-core/src/records.rs)
when only the renderer does. Packers fill the typed record and cast it with
`bytemuck`; a partial toggle writes at an `offset_of!` taken from the record,
never at a hand-kept offset. `tests/pipeline_abi.rs` checks
each record's member names, offsets, spans, nested records and array counts
against every built-in shader, and `every_buffer_struct_has_a_checked_layout`
fails when a shader reads a struct that is not a record. The only exemptions
(`BUFFER_LAYOUT_EXCEPTIONS`) are GPU-written or one-word formats.

**Every fixed length and mask in WGSL comes from its Rust owner.**
[`shader_constants`](../../../crates/presentation/src/shader_constants.rs) is
prepended to every built-in shader. Two concepts that share a value keep two
names (the scar `KEY_MASK` and the ground field's `RECORD_MASK` share a
value). Raising `WAVE.max_steps` therefore resizes the shader's splat
array with it. The fixed index arithmetic of the indirect-draw arguments (five
words a draw) is the WebGPU format, not a tunable length.

**The generated TypeScript records are a chosen subset.** `records/typescript.rs`
exports explicit lists of what the page reads, not the whole schema, so adding a
GPU record adds nothing to the browser.

**Shader text has one authored body per algorithm, in its concept's module.**
[`shaders/README.md`](../../../crates/renderer-core/src/shaders/README.md) owns
the composition grammar (`shader|block,block@N`, where `@N` supplies a shared
binding's group), what the build refuses, and the concept-to-module map.
Nothing rewrites
shader text at runtime: the Windows cutout workaround is authored as
`…Discard` shaders, picked with `cfg!(windows)` in the model registry.

**Grass draws in the same order every time.** Grass placement counts each
tile's survivors, prefixes their ranges, then writes every clump to its slot
(entry points `grassCount`, `grassFinish`, `grassBuild` in
`shaders/modules/passes_grass.wgsl`, dispatched in that order by
[`grass.rs`](../../../crates/renderer-core/src/grass.rs)). Draw order and
which clumps a full tier keeps no longer depend on GPU scheduling, so a parity
run renders byte for byte. The count and prefix passes bind only what they read, so
every pipeline stays within WebGPU's default per-stage limits (eight storage
buffers); a CPU-only registry test fails any pipeline that would exceed them,
because the device requests only what the adapter reports. This is what makes the plate proof possible at all.

**Scene admission is Rust's.** Both shells place scenery through
`renderer_core::scenery::place_scenery`, build world geometry through
`WorldGeometry::from_triangles` / `from_mesh`, and read each model node's track
side from presentation (`Rig::track_side`); TypeScript passes inputs and
rebuilds none of them. TypeScript keeps one copy of the track rule,
`articulationRig` (`packages/scene-assets/src/articulation.ts`): the model
build's validator needs it to reject a malformed file before Rust sees it, and
the lab and bake posers use the same rig.

**GPU frame timing has one owner.** [`timing.rs`](../../../crates/renderer-core/src/timing.rs)
`FrameTimer` resolves `Resolve::InFrame` for the browser and
`Resolve::AfterCompletion` for native's bench (Metal can return stale end timestamps in-frame). The browser
stays in-frame because it measured the same in both modes.

**The renderer's tests live in renderer-core.** `cargo test -p renderer-core`
runs its GPU suites with one device helper (`tests/support`). Native keeps the
tests that need its replay, filesystem library or effect loader, plus capture,
audio and bench.

## How it was proved

Each structural change had to leave every plate byte-identical. The plates came
from native renders of three frozen browser parity runs: the menu reel, the
menu reel with fog of war, and the water reel (54 plates). Before rendering, a
cheaper check compared the assembled WGSL of the base and changed builds: an
empty diff proves a pure move with no GPU. The scripts lived in the ignored
scratch folder (the user chose a scratch script over a committed tool).

The first probe failed. The original base build did not render grass the same
way twice: scattered grass pixels varied between runs and took more than two
values, so no tolerance could separate noise from change. Fixing the grass
order (above) restored exact reproduction. Its own picture change, against the
old base, is the same size as the old build's run-to-run noise: up to 44 levels
on at most 0.3% of pixels, all on grass blades. An unprimed critique saw no
difference ([before](assets/grass-order-before.png),
[after](assets/grass-order-after.png), [changed pixels](assets/grass-order-mask.png)).
Every later plate proof compares against the grass-fixed build.

## Dead ends

- **A loop over the shadow cascades.** It assembled back to the same tokens,
  but the loop index stopped the compiler folding literal cascade indices and
  moved one wetlands pixel by one level. The blend stays unrolled.
- **Proving plates with a tolerance.** Grass noise took several values per
  pixel, so neither a threshold nor "matches one of the base renders" held. The
  fix was determinism, not a looser check.
- **Packing records by writing words at layout offsets.** It added lines
  without making the record the owner; packers fill typed records instead.
- **Bridging TypeScript posers to Rust's track rule through wasm.** It added a
  JSON transport and a cache while the TypeScript rule stayed for the model
  build's validator, so it removed no copy.
- **A non-blocking after-completion timing path.** No caller used it.
- **A JavaScript copy of shader composition** for the V-wake check. It broke on
  the next rename; the check is a Rust GPU test on the real pipeline
  (`tests/water_wake.rs`).

## V-wakes

The water spec promised "rings, plus a V-wake once the unit moves"
([decision 13](../water-and-wetlands/decisions.md)). Fast movers in deep water
already dropped a trail of V-wake splats, but the shader drew every splat as a
ring and no V showed ([before](assets/v-wake-before.png)). The wave splat now
carries the mover's heading, and the shader draws V-wake splats as two
trailing arms ([after](assets/v-wake-after.png)); ring splats are unchanged.
The user signed off on the after shot. `tests/water_wake.rs` pins the
behaviour on the production wave-step pipeline: energy behind the travel,
both arms present, rings symmetric.

## Pointers

- Records and their checks: `crates/presentation/src/records.rs`,
  `records/layout.rs` (`records!`), `crates/renderer-core/src/records.rs`,
  `crates/renderer-core/tests/pipeline_abi.rs`.
- Shader composition: `crates/renderer-core/build.rs`,
  `src/shaders/pipeline_sources.txt`, `src/shaders/README.md`,
  `src/pipeline_registry/tests.rs` (every binding against its layout, both
  cutout variants).
- Grass determinism: `crates/renderer-core/src/grass.rs`, `tests/grass.rs`.
- Scene admission: `crates/renderer-core/src/scenery.rs`,
  `src/runtime/world.rs`, `src/models/admission.rs`,
  `crates/presentation/src/world.rs` (`TrackSide`).
- Timing: `crates/renderer-core/src/timing.rs`, `tests/frame_timing.rs`,
  `native/client/src/bench.rs`.
