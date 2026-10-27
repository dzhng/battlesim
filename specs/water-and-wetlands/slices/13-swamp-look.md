# 13: Swamps drawn as water

**Lane:** wet. **Depends on:** 05 (the soft shore) and 10.
**Visual variable:** the look of a swamp with units standing in it.

## Contract

- **One standing-water owner:** `StandingWater`, in
  `crates/presentation/src/static_world/ground/water.rs`, built from the
  exported rivers and swamps.
  - It answers `at(xy) -> Option<Wet { surface_z, depth_m, look }>`.
  - It serves drawing here, and dust suppression and wakes in slice 15. No
    second water lookup is created.
- **Swamp cells** join the one `WaterCell` list, with swamp `WaterLook` rows:
  flooded meadow and low reed marsh (decision 10). Each swamp is picked by
  slice 03's `look(seed, feature)`.
- **Reeds** come from the existing grass and dressing owner, with their height
  capped in data at knee height (decision 6). There is no second swamp shader
  and no new scatter system.
- **Depth:** tuned on screen with soldiers standing in the swamp, so the water
  reaches their thighs (decision 30). The value is data.
- **Posture:** a wading soldier is never drawn lying down. The presentation's
  pose stance skips prone where the standing water is deeper than a lying
  soldier's height (decision 33). That is a physical comparison, not a swamp
  rule.

## Verification

1. A presentation test: a soldier standing in water deeper than a lying
   soldier is never posed prone.
2. A presentation test: a swamp's look depends only on the seed and the
   swamp's index.
3. Run `compare-screenshots` on the swamp mask with units in it, at the close
   and tactical framings, against `../assets/redgpu-wading-shore.jpg` and
   `redgpu-wake-rings.jpg`, which show a unit standing in water.
4. Run `screenshot-critique` last, for each swamp look.
5. Non-blocking checkpoint: show both looks with `preview-shots`. Wait about 5
   minutes. If the user stays silent, decide, record the decision and continue.

**Out of scope:** wakes (slices 15 and 16) and puddles (14).

## Native

Free through presentation and the shared material.

## Delegated to the implementer

- reed geometry and density;
- each look row's colours.

## Must stay green

Battle digests (this slice changes the look only) and the river crops.
