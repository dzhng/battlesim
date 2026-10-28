# 13: Swamps drawn as water

**Lane:** wet. **Depends on:** 05 (the soft shore) and 10.
**Visual variable:** the look of a swamp with units standing in it.
**Status:** done, reeds open (2026-10-28).

## Contract

- **One standing-water owner:** `StandingWater`, in
  `crates/presentation/src/static_world/ground/water.rs`, built from the
  exported rivers and swamps (and, from slice 14, the puddles).
  - It answers `at(xy) -> Option<Wet { surface_z, depth_m, look }>`. Water
    stands where its surface lies above the ground, so a road across a
    swamp, which the simulation leaves uncarved, is dry by depth alone.
  - It serves drawing, the plots (`flooded`), the posture rule, and slice
    15's dust and wakes. No second water lookup is created.
- **Swamp cells** join the one `WaterCell` list, with the biome's swamp
  `WaterLook` rows after the rivers': flooded meadow and low reed marsh
  (decision 10). Each swamp is picked by the seed, the kind and its index
  (`look(seed, Kind::Swamp, k)`).
- **No crop stands in a swamp:** fields are cut along a swamp's outline as
  along a river's run, and a plot inside it takes the plot kind its look
  names (`ground`: meadow, or rough for the marsh).
- **Reeds:** not built. The marsh grows rough grass under its water ([open](../README.md#still-open)).
- **Depth:** the wetlands map's 0.6 m (decision 30) stands, judged on screen
  with a squad wading in it.
- **Posture:** a wading soldier is never drawn lying down (decision 33).
  Where he would kneel or lie, he takes the first posture, from his own
  upward, whose chest stays above the water at his feet
  (`presentation.pose.stance.chest_m`): in the 0.6 m swamp he kneels, in
  deeper river water he stands.

## Verification (done)

1. A presentation test: a soldier standing in water deeper than a lying
   soldier is never posed prone, and on dry ground the same soldier is
   (`tests/wading.rs`, in the swamp and in a river).
2. A presentation test: a swamp's look depends only on the seed and the
   swamp's index, whatever else the map holds.
3. A presentation test: no crop stands in a swamp. It fails with the rule
   taken out.
4. A presentation test: a road across a swamp stands dry.
5. `compare-screenshots` on the water reel's new swamp shots (a squad in
   the wetlands swamp at 45 m, and wading at 12 m), each look pinned
   (`SWAMP_LOOK=<row>`), against `../assets/redgpu-wading-shore.jpg` and
   `redgpu-wake-rings.jpg`. The first shot showed wheat growing through the
   water, which the crop rule fixed.
6. An unprimed `screenshot-critique` of both looks found:
   - the water reads knee-deep, though it is the full 0.6 m, just above a
     soldier's knee;
   - nothing stands out of the water (reeds, open);
   - no rings at the legs (slices 15 and 16);
   - the meadow reads close to dry from above, so its flood was tinted.
7. Non-blocking checkpoint: both looks, from above and wading, shown in
   Preview, with no feedback. Decided: both looks stand, the depth stays,
   reeds stay open.
8. Codex found swamp water drawn over causeways, a second swamp lookup in
   the plots, one swamp height for all in the scene, and a vacuous test; all
   fixed. Then, on the posture, bridges counting as water; see slice 15.

## Native

Free through presentation and the shared material.

## Must stay green

Battle digests (this slice changes the look only) and the river crops.
