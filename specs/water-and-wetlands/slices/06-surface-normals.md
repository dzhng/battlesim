# 06: The water surface: wind-driven normals

**Lane:** render. **Depends on:** 01 and 05.
**Visual variable:** surface detail and motion.
**Status:** done (2026-10-28).

## Contract

- **The normal texture is generated once at load** by an exported bake
  pipeline, next to `bake.wgsl` (decision 16), with one lifetime owner. No
  image files ship, and RedGPU's unlicensed maps are not used.
- **The shader samples it at three scales:** primary, a rotated macro layer and
  a rotated detail layer. They blend with RNM, each scrolling along the
  slice 01 `Wind` at its own rate, and fade toward flat with distance
  (RedGPU `lakeFragment.wgsl:280-346`).
- **Deleted:** `waterNormal`'s procedural ripple octaves.
- **Vertex waves:** judged at the low framing. They are dropped if they don't
  read.

## What changed from the plan

- The map has no mips; each layer flattens as a pixel outgrows its texels
  ([choices](../choices.md)).
- Vertex waves were not built: centimetres don't read from an RTS height.
- The streaks of light along the stream stay, for slice 09 to judge.

## Verification (done)

1. `compare-screenshots` on the water at all three framings, against slice
   05's plates and `../assets/redgpu-overhead-shadows.jpg`:
   - Low: the even field of procedural ripples is replaced by waves at
     three scales, which break the sun's glint into many facets.
   - Close and tactical: little change from a steep camera, where the
     streaks and the depth colour own the look.
   The motion pair (two plates 0.49 s apart, from a throwaway two-shot reel
   in the graphics test) shows the surface drifting downwind. Verdict: less
   wrong at the low framing; even at the steep ones.
2. Pausing freezes the surface: a paused lab's frames are identical,
   because its clock stops with the battle.
3. Cost: the bake is one 256×256 full-screen draw at load. Per frame,
   interleaved against a flat normal at the tactical framing in the river
   lab: High 9.21 ± 1.09 ms against 9.29 ± 0.91 ms, Low 6.410 ± 0.033 ms
   against 6.419 ± 0.033 ms. Within noise.
4. An unprimed `screenshot-critique` of the still and the motion pair, on
   the first tuning, found blown-out glints and a soft near field (tuned),
   streaks reading as brushed metal (slice 09), and a surface that slides
   rather than churns (the three layers' drift rates now differ more).
5. The water, river and frame scenes pass. The frame baselines are
   unchanged: their compositions are far enough out that the normals fade
   flat.
6. `fixtures/parity/exact-numbers.json` was re-recorded for the renamed
   tuning (`ripple` became `normals`); its digest rows are unchanged.
7. Codex found the turned layers' slopes used as if along the map's axes;
   they are now turned back before they lean the normal.

## Native

Ported: native replays the exported bake once (`water_normals.rs`) and
binds the map with the terrain's tables.

## Delegated to the implementer

- texture resolution and the generator;
- the scroll rates per layer;
- whether to keep vertex waves.

## Must stay green

Digests, replays, and slices 04 and 05's crops when motion is frozen.
