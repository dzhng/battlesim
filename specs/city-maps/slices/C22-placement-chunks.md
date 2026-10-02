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

## Outcome

**First half, 2026-10-01: one static chunk owner, with corpses on it.** Buildings from kits (template references, residency, the pool) have not started; they wait on C32's library.

- **The owner** is `packages/battle-renderer/src/frame/staticChunks.ts`. A population hands it records of any stride with each instance's kind, size and bounds; it keeps them in chunk order and, per view, gives the merged ranges of chunks drawn whole at a level, the near chunks, and the sun's caster ranges. A level is whatever the population's layer draws for it, and a population says how many it has.
- **Scenery** (`scenery/lod.ts`, `frame/sceneryLayer.ts`) is three populations of it, drawn as before: four mesh tiers, a far chunk whole at the last.
- **Corpses** (`models/modelDetail.ts`, `models/modelLayer.ts`) are a fourth: one kind, the models' 16-float record, and a fifth level, the impostor card, for a chunk whose every corpse has a card. Near chunks are chosen per corpse by the models' own detail rule and packed with the units. The private corpse chunking is deleted.
- **What the next pass builds on:** a kit module population is a `ChunkSource` and a `ChunkLevel`; a far-tier tile is another level. Neither needs a change to the owner's selection.

**Proof that nothing drawn changed.** A differential probe ran the base commit's two chunk paths against the owner on seeded random populations: 800 scenery views (327,000 staged instances, 13,700 far ranges, 6,300 cast ranges) and 2,000 corpse views (18,000 card runs, 21,000 near chunks) agreed exactly, buffers and chunk boxes included. In the browser on the real GPU, the endurance lab's late state (1,000 corpses drawn) at eight cameras from 25 m to 2.4 km and a generated town at seven were captured from the base commit and from the change with grass, effects and cast lights off: every frame is pixel-identical (0 of 2,073,600 pixels differ in each of 15 frames) and the model and scenery stats are equal. The decisions are in [choices](../choices.md#c22-static-chunk-owner).
