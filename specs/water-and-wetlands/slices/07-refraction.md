# 07: Refraction, and the Low tier

**Lane:** render. **Depends on:** 06.
**Visual variable:** the bed seen displaced through water.

## Contract

- **Refraction:** the offset is the difference between the perturbed normal
  and a flat one, scaled by depth over camera distance and clamped. It is
  rejected where it would sample foreground geometry, and the colour copy's
  mip level rises with depth (RedGPU `lakeFragment.wgsl:380-413`).
- **Fog:** a sample is taken only where the fog mask copy says the sample point
  is seen. That stops a seam at the fog edge.
- **The Low tier:** `GraphicsQualityProfile.waterDetail: "full" | "depth"`
  (`apps/battle-lab/src/graphicsSettings.ts`) reaches
  `BattleFrameOptions.quality` and lands as one uniform word. It turns
  refraction off on Low (decision 18), and slices 08 and 16 read it too. When the copy isn't needed, a 1×1 stub texture is bound.
  There is one pipeline for every tier.

## Verification

1. Run `compare-screenshots` on the water mask at the close and low framings,
   against `../assets/redgpu-wading-shore.jpg` and slice 00's stills.
2. Check a crop at the fog edge: no seam, and no unseen pixel shows through.
3. Low's shot is unchanged from slice 06's Low shot.
4. Perf: the water pass's GPU time on Low against High, at the tactical
   framing, interleaving runs.
5. Run `screenshot-critique` last, on the water crops.

**Out of scope:** caustics (slice 08) and highlights (09).

## Native

Free with slice 02's copy. Native runs at `full`. If native can't bind the mip
chain, it runs `depth` until a
[`water-native`](../../water-native/README.md) item removes that.

## Delegated to the implementer

- the refraction scale and clamp;
- how mips are generated.

## Must stay green

Digests, replays, and every Low shot.
