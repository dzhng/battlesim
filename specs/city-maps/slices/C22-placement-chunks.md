# C22: placement chunks

**Depends on:** C32, C01. **Kind:** slice.

## Question
Do reusable template instances draw within bounded residency at full extents, with no per-module CPU work per frame?

## Contract it unlocks
- **One static-chunk owner** for storage, residency, tiering and casters, **promoted from the scenery layer's existing chunk path** (`frame/sceneryLayer.ts`, `scenery/lod.ts`), which already draws trees, hedgerows and (from C79) forest dressing at kilometre scale. **Corpse chunks** (`modelDetail.ts:66-110`) **move onto it in the same slice**, and kit modules and C23's far-tier tiles join it. Materials stay per pass; there is one chunk owner (S-chunks).
- Keep compact placed-building/template references indexed over the world. Cull/select resident chunks before expanding C32's local module placements. Expand once on chunk entry and stream through a fixed pool; never materialize all module transforms for a full map. Saved and runtime maps use the same resolver/residency path. Map replacement and edge-to-edge moves obey S3/G0's limits.
- A building's appearance is `drawn_by: "placements"`. The knowledge rule is unchanged: a side draws a building's ruin state only once it has seen the collapse.
- Opaque only.

## API seam
`packages/battle-renderer/src/{frame/sceneryLayer.ts,scenery/lod.ts}`, `models/{modelLayer.ts,modelDetail.ts,propAppearance.ts}`.

## What the human can run or see
`/lab/city-block` (a real block at the ground, default and strategic cameras) and `/city-scale` benchmark rows.

## Verification
- Byte-stable template resolution against C32 for saved/runtime instances, with correct local/world frames and learned damage state.
- Resident expansion/pool bounds hold during pan/zoom/map replacement; measure world reference bytes and cold startup separately.
- Benchmark: resident instances and bytes, prepare ms, triangles, worst-window GPU within G0's budget.
- Corpse scenes unchanged.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**facade density and massing at 80 m; exclude windows, materials' fine detail and ground**) against **C13's frozen template contact sheets and C53's accepted generated block overlay**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Glass, interiors (C24–C26), far tier (C23), streets (C28+).


## Delegated to the implementer
Residency radius within G0's; pool size; record packing. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Village visuals while its existing house path is awaiting C37; corpse behaviour. C37 removes that short-lived house path when the original shapes resolve through the template library.

## Feedback that would change this slice
Poor residency transitions or expensive expansion reopens chunk/pool policy within the measured G0 limits.
