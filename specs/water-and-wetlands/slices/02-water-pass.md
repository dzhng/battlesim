# 02: The water pass

**Lane:** render. **Depends on:** 00.
**Visual variable:** none. Pixels must be unchanged except at water edges.
**Status:** done (2026-10-28).

## Contract

The frame order is now:

1. the world: opaque geometry, grass, the backdrop, **and glass**, resolved
   into `lit` and `fogMask`, **keeping its samples** (`storeOp: "store"`);
2. **water**, in a pass of its own over those samples, with the world's depth
   attached read-only, resolved into `lit` and `fogMask` again;
3. effects.

`packages/battle-renderer/src/frame/worldPass.ts` (`encode`) owns that order,
and `battleFrame.ts`'s header names it. Native mirrors it: `WorldColour`, then
`world::Water`, in `native/client/src/main.rs`'s pass list.

- **Water stays multisampled** (decision 25, settled by the probe below). The
  `worldWater` pipeline's export is unchanged, so native's pipelines stand.
- **Glass stays in the world pass, before the water.** Water seen through a
  pane draws over the pane: the accepted cost.
- **The world pass still resolves.** Metal skips the resolve of a tile the
  water's pass leaves empty, so `lit` there must already hold this frame.
- **The water material is unchanged.**

## What changed from the plan

- **Multisampled, not single-sampled.** The probe found storing the world's
  samples costs less than run-to-run noise, which is decision 25's condition
  for keeping the samples. So the water keeps its anti-aliased edge and its
  hardware depth test. No in-shader depth test is needed yet.
- **No scene-colour copy or fog-mask copy yet.** Nothing reads them until
  slice 04, so they arrive there ([`choices.md`](../choices.md)).
- **A pipeline binds at most four groups,** and the water already binds four:
  camera, terrain, environment and fog. Chrome reports `maxBindGroups` 4 on
  this Mac, the WebGPU default. A later slice that reads the depth or a copy
  in the water's shader must fit them into one of those four. A single-sampled
  build tried here put the depth in the fog layout, with a 1×1 placeholder in
  every group the world pass binds, because that pass writes the depth. It
  worked on both clients. Slice 04 inherits the finding.

## Cost probe (decision 25)

Whole-frame GPU time (`frame/gpuTiming.ts`, the only total Apple's GPU makes
meaningful) at the tactical framing (260 m, 0.95 rad, over the river lab's
wide stretch, 1920×1080, fog off). The arms were (a) single-sampled water
after the resolve and (b) the samples kept, water multisampled. Eight
interleaved arms of 120 frames each, on one page per tier:

| tier | (a) mean ± sd | (b) mean ± sd |
| ---- | ------------- | ------------- |
| High | 6.525 ± 0.049 ms | 6.539 ± 0.057 ms |
| Low  | 8.18 ± 0.81 ms | 8.05 ± 0.90 ms |

(b) is within noise of (a) on both tiers. Low's arms drifted together under
other load on the machine, and its frame was slower than High's.

## Verification (done)

1. The `crates/pipelines` tests pass, and so does the shader-export test,
   with the export unchanged.
2. `native/client/tests/pipelines.rs` builds every pipeline.
3. The water scene passes on both clients. Native's counts equal the
   browser's, and the river reads as bluer than its bank, as it did before.
4. Before against after: the `river` stills (paused lab frames) are the same
   but for a few scattered pixels. The single-sampled trial changed the
   waterline and the water's edge at the bridge's piers. Its stair-stepped
   waterline was judged, and dropped with the probe.
5. Glass: no vehicle on the water's framings has a pane in front of water. The
   case is accepted by decision 25.
6. An unprimed `screenshot-critique` of the edge crops, old against the
   single-sampled trial, found no depth-ordering error: soldiers, the tank's
   barrel and the bridge all draw over the water. It found the trial's
   waterline stair-stepped, which the multisampled pass avoids. Its notes on
   the look belong to later slices: a thin orange fringe at the waterline
   (05), streaky ripples (06), and flat colour (04).

## Native

Ported: `world::Water` follows `WorldColour`, which now keeps its samples.

## Must stay green

Battle digests and replays, every scene outside the water mask, and resource
teardown after resize and reset.
