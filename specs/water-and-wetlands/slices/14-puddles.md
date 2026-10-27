# 14: Puddles

**Lane:** wet. **Depends on:** 13. **Visual variable:** the puddle look.

## Contract

- **Puddles are look only** (decision 29). They are not in the map contract,
  have no dip, and change no digest.
- **Placement is a pure function**, `puddle_at(xy)`, of the biome seed and the
  ground walk's cell. It must not depend on what the walk has already laid
  out, so wakes, grass and both clients get the same answer on any camera
  (decision 34). It is scattered on tracks and fields from biome data, reusing
  the scatter that plots and scenery already use.
- **The grass and dressing walk read the same function,** so nothing grows in
  a puddle.
- **Look:** mirror on `road` and `paving`, muddy on `country_road`,
  `dirt_track` and open ground, judged at the puddle's centre (decision 28).
- **Depth:** puddles don't carve, so their depth for colour and shore comes
  analytically from their shape. That is a per-cell input to the one water
  material, not a second material.
- `StandingWater.at` includes puddles, so slice 15's wakes and dust see them.

## Verification

1. A presentation test: a puddle's look follows the surface under its centre.
2. A presentation test: placement is stable for a seed whatever the walk order.
3. Battle digests are unchanged.
4. Run `compare-screenshots` on puddle masks on a road and a field at the close
   and tactical framings, against our own before shot. It has no RedGPU
   counterpart.
5. Run `screenshot-critique` last.

## Native

Free.

## Delegated to the implementer

Density, size range and shape noise.

## Must stay green

Battle digests and replays, and grass and road crops away from puddles.
