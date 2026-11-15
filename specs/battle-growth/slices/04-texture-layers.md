# 04 Texture layers

**Question:** can unchanged model textures survive a swap without being
written again, with no shader change and no higher memory peak?

**Depends on:** 03.

## What happens today

Each generation builds two array textures (colour and surface) from every
texture it draws, upscaling small ones on the CPU, and
[`Drop for Models`](../../../crates/renderer-core/src/models.rs) destroys them
outright. Layers are numbered in first-seen order. During a swap both
generations hold a full copy.

## Contract

- A model texture is written to the GPU once per scene.
- A retiring generation cannot destroy what another holds.

## Seam

- **Shared texture:** a handle destroyed by its last holder. It replaces the
  explicit destroy in `Drop for Models`.
- **Layer slots:** assigned per texture id by the art store, append-only.
- **The arrays carry spare layers.** A layer is written once, before any
  committed materials table names it. Pending work writes only layers no
  committed table names; that is how the placeholder's rule, never overwrite a
  shared resource during preparation, is kept.
- **Growth:** when the spare layers run out, a larger array is made and the
  live layers are copied GPU-side. A texture with a larger edge than the array
  rebuilds that array; the report counts it as such.

## Verification

Write first; test 1 is red today.

1. Commit {x, y}, then stage {a, x, y}: texture layers written equal `a`'s new
   textures only.
2. Two holders of one shared texture: drop one and a readback still succeeds;
   drop both and the renderer's memory report returns to its baseline.
3. The committed frame is pixel-equal to a clean build, including after array
   growth and after a larger texture arrives.
4. A pending generation that wrote layers and is then dropped leaves the live
   picture unchanged.
5. Memory during the overlap equals the live generation plus the new art, and
   the growth peak is no higher than today's two-generation overlap.

## What you can run or see

`battle-growth`: texture work at growth is the new type's. The page's GPU
memory figures across a swap no longer show two full texture sets.

## Delegated

The growth step, and whether layers written by a dropped pending generation
are reused or left dead.

## Stays green

`tests/bundle_memory.rs`, `tests/impostors.rs` (the bake reads these arrays),
the texture-channel diagnostics, device recreation.

## Feedback that would change this slice

If test 5 fails, shrink the spare capacity before anything else.
