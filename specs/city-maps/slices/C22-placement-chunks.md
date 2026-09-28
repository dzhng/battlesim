# C22: placement chunks

**Depends on:** C13, C01. **Kind:** slice.

## Question
Do baked placements draw as instanced kit modules at the frame budget, with no per-instance CPU work per frame?

## Contract it unlocks
- **One static-chunk owner** for storage, residency, tiering and casters, **promoted from the scenery layer's existing chunk path** (`frame/sceneryLayer.ts`, `scenery/lod.ts`), which already draws trees, hedgerows and (from C79) forest dressing at kilometre scale. **Corpse chunks** (`modelDetail.ts:66-110`) **move onto it in the same slice**, and kit modules and C23's far-tier tiles join it. Materials stay per pass; there is one chunk owner (S-chunks).
- Buildings are culled before their modules are visited (`modelLayer.pack`, `:1184`). The tier is chosen per chunk. Instance ranges are uploaded once, and tiles stream in and out of a fixed pool.
- A building's appearance is `drawn_by: "placements"`. The knowledge rule is unchanged: a side draws a building's ruin state only once it has seen the collapse.
- Opaque only.

## API seam
`packages/battle-renderer/src/{frame/sceneryLayer.ts,scenery/lod.ts}`, `models/{modelLayer.ts,modelDetail.ts,propAppearance.ts}`.

## What the human can run or see
`/lab/city-block` (a real block at the ground, default and strategic cameras) and `/city-scale` benchmark rows.

## Verification
- Byte-stable placement parity against C13.
- Benchmark: resident instances and bytes, prepare ms, triangles, worst-window GPU within G0's budget.
- Corpse scenes unchanged.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**facade density and massing at 80 m; exclude windows, materials' fine detail and ground**) against **`throwaway/evidence/city-maps-chiro/sheet.jpg` 80 m column and an aerial crop of the real block**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Glass, interiors (C24–C26), far tier (C23), streets (C28+).


## Delegated to the implementer
Residency radius within G0's; pool size; record packing. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Village visuals (house GLBs untouched); corpse behaviour.
