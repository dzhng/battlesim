# 02: The water pass

**Lane:** render. **Depends on:** 00.
**Visual variable:** none. Pixels must be unchanged except at water edges.

## Contract

The frame order becomes:

1. the world: opaque geometry, grass, the backdrop, **and glass**;
2. the MSAA resolve into `lit`;
3. copy the scene colour, with mips, and the fog mask;
4. **water**;
5. effects.

That order is owned by `packages/battle-renderer/src/frame/worldPass.ts` and
`battleFrame.ts` alone.

- **Water is single-sampled into `lit`** (decision 25). It tests depth in its
  shader against the existing multisampled `targets.depth`, reading sample 0
  with `textureLoad` on `texture_depth_multisampled_2d`, as `effectPass.ts`
  already does. No depth copy or resolve is added, so native's export schema
  gains no new value types.
- **Glass stays in the MSAA world pass** (decision 25, revised). The cost is
  accepted: water seen through two panes of glass is drawn over the glass.
- **New targets in `frame/targets.ts`:** `sceneColour` (with a mip chain) and
  a copy of the fog mask, each with one lifetime owner. `lit` gains whatever
  usage the copy needs. Resolve into `lit` itself, never into a new target:
  Metal skips empty tiles and would keep a stale frame's pixels.
- **The water material is unchanged in this slice.** It doesn't yet sample the
  copy; that starts in slice 04.

## Cost probe (decides decision 25)

Measure GPU time per pass with `frame/gpuTiming.ts`, at the tactical framing,
on High and Low, interleaving A and B runs. Compare:

- (a) single-sampled water after the resolve;
- (b) keeping the MSAA samples (`storeOp: "store"`) and drawing water MSAA.

Record the numbers in the README. Keep (a) unless (b) costs less than run-to-run
noise.

## Verification

1. `crates/pipelines` tests (`naga`, `uniforms`, `sizes`) pass.
2. The shader-export test passes.
3. `native/client/tests/pipelines.rs` builds the changed `worldWater`.
4. The slice 00 native draw check passes.
5. Run `compare-screenshots` on the `water` and `river` scenes, before against
   after. Nothing may change outside the water-edge mask. At the edge,
   single-sampling is an expected, named change; judge it, and don't loosen
   any check.
6. Check a crop of a vehicle by the river for glass.
7. Run `screenshot-critique` last, on the water-edge crops.

## Native

Must be ported in this slice. `worldWater`'s sample count and targets change.
Keeping an old in-pass water pipeline for native would be a compatibility
layer, which is forbidden.

## Delegated to the implementer

- whether the copy is a blit draw or `copyTextureToTexture`;
- how many mip levels;
- how the in-shader depth comparison follows `depthContract.ts`.

## Must stay green

Battle digests and replays, every scene outside the water mask, and resource
teardown after resize and reset.
