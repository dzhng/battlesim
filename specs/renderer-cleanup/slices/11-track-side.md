# 11 · Track side: decided once, in presentation

**Contract.** Whether a model node is a left or right track is decided by
presentation, and the renderer reads that decision.

## Today

- `crates/renderer-core/src/models/admission.rs:126-130` decides by name only.
- `crates/presentation/src/appearance.rs:357-360` requires the name and
  `link_pitch_m > 0`.
- TypeScript `articulationRig` (`packages/scene-assets/src/articulation.ts:256-261`)
  repeats presentation's rule; the bake validator uses it
  (`validate.ts:925-937`) and so do the workbench's live posers.
- No disagreement today: the bake rejects pitchless tracks.

## Seam

- Presentation exposes each node's track side (`Option<Side>`) from its
  pitch-qualified rig facts; `Models::from_world`/admission reads it by node
  index. The renderer's `bundle::Node` (`bundle.rs:88-93`) carries no extras, so
  it can't decide this alone.
- The TypeScript bake validator stays: it rejects a malformed file before Rust
  sees it, a different job (map unknown 9). Trace `articulationRig`'s other
  callers; a live runtime caller that reads track sides reads presentation's
  fact through wasm.

## Verify

- Failing first: a model with `track_L` but no positive `link_pitch_m` is not
  admitted as a track by the renderer.
- `cargo test -p renderer-core` (bundle admission); menu reel byte-identical
  (it has a tank).

**Delegated:** the type and accessor names.
**Stays green:** track scroll and link shading.
