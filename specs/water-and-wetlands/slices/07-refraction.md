# 07: Refraction, and the Low tier

**Lane:** render. **Depends on:** 06.
**Visual variable:** the bed seen displaced through water.
**Status:** done (2026-10-28).

## Contract

- **Refraction:** the bed point moves along the perturbed normal's slope by
  a share of the depth it is seen through. The depth is capped, and the
  shift is projected to the screen and clamped. The shift is dropped where it
  would sample foreground geometry. The colour copy's mip level rises with
  the water it is seen through (RedGPU `lakeFragment.wgsl:380-413`),
  through `waterUnder` (`frame/waterUnder.ts`).
- **Fog:** a sample is taken only where the fog mask copy (`fogMaskUnder`,
  bound as `fogLayout.seenUnder`) says it is seen at least as well as the
  water's own pixel. That stops a seam at the fog edge.
- **The Low tier:** `GraphicsQualityProfile.waterDetail: "full" | "depth"`
  reaches `BattleFrameOptions.quality` (`WaterDetail`, `frame/targets.ts`)
  and lands as one uniform word, `TerrainParams.view.z` (1 for `depth`). It
  turns refraction off on Low (decision 18), and slices 08 and 16 read it
  too. On `depth` the targets build no mips and no fog mask copy, and the
  fog's 1×1 placeholder is bound in the copy's place. There is one pipeline
  for every tier.

## What changed from the plan

- The shift is a world-space shift, projected, rather than RedGPU's
  view-space UV offset, and it is eight times the physical one so that it
  reads at the close framing ([choices](../choices.md)).
- The cost probe measured a pair on High, with refraction on and off,
  because the tiers' totals differ in everything else.

## Verification (done)

1. `compare-screenshots` on the reel's three framings, against a run of this
   build with no shift and no blur (slice 06's water; slice 06's saved plates
   predated its last retune), and `../assets/redgpu-wading-shore.jpg`:
   - Close: the shallows' smooth gradient breaks into a wobbling, displaced
     bed. The waterline under the soldiers is unchanged, with no seam or
     doubled edge.
   - Tactical: unchanged; the shift stays under a pixel at 260 m.
   - Low: small changes in the near shallows only.
   The blur alone moved no visible pixel: the bed shows only in the
   shallows, and has no detail there yet. Against the reference, ours still
   shows no bed through the shallows: no stones or texture for the
   refraction to bend. Verdict: less wrong.
2. Fog edge: the reel filmed with the side's fog (`?fog=1`) shows the fog
   crossing the river by the bridge identically with the shift on and off.
   There is no seam, and nothing unseen shows through.
3. Low: the frame scene's Low baselines (river, geometry) differ by 0 px.
4. Cost, interleaved, eight arms of 120 frames at the tactical framing in
   the river lab: High 9.17 ± 0.72 ms refracting against 9.24 ± 0.84 ms
   without, within noise. Low on the same run was 10.47 ± 0.64 ms.
5. An unprimed `screenshot-critique` of the three framings, their crops and
   the fog shot found no defect of this slice's. Its findings belong
   elsewhere:
   - no visible bed through the shallows (bed detail, with 08's caustics);
   - streaks reading as brushed metal (09);
   - a pale band and line at the waterline (05's shore);
   - the bridge's piers with no contact with the water (15 and 16);
   - fog over the river stepping, without a rim, near the bridge (the
     fog's own, unchanged here).
6. The water scene passes on both clients, native within ΔE 0.0 of the
   browser. `fixtures/parity/exact-numbers.json` was re-recorded for the
   new tuning.

## Native

Ported: native allocates the mips and the fog mask copy, replays
`sceneColourMip` and the copies in `world::Water`, and binds the copy and the
sampler in the water's fog group. It draws `full`.

## Must stay green

Digests, replays, and every Low shot.
