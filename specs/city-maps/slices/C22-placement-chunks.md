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

**First half, 2026-10-01: one static chunk owner, with corpses on it.** Buildings from kits (template references, residency, the pool) are the second half, below.

- **The owner** is `packages/battle-renderer/src/frame/staticChunks.ts`. A population hands it records of any stride with each instance's kind, size and bounds; it keeps them in chunk order and, per view, gives the merged ranges of chunks drawn whole at a level, the near chunks, and the sun's caster ranges. A level is whatever the population's layer draws for it, and a population says how many it has.
- **Scenery** (`scenery/lod.ts`, `frame/sceneryLayer.ts`) is three populations of it, drawn as before: four mesh tiers, a far chunk whole at the last.
- **Corpses** (`models/modelDetail.ts`, `models/modelLayer.ts`) are a fourth: one kind, the models' 16-float record, and a fifth level, the impostor card, for a chunk whose every corpse has a card. Near chunks are chosen per corpse by the models' own detail rule and packed with the units. The private corpse chunking is deleted.
- **What the next pass builds on:** a kit module population is a `ChunkSource` and a `ChunkLevel`; a far-tier tile is another level. Neither needs a change to the owner's selection.

**Proof that nothing drawn changed.** A differential probe ran the base commit's two chunk paths against the owner on seeded random populations: 800 scenery views (327,000 staged instances, 13,700 far ranges, 6,300 cast ranges) and 2,000 corpse views (18,000 card runs, 21,000 near chunks) agreed exactly, buffers and chunk boxes included. In the browser on the real GPU, the endurance lab's late state (1,000 corpses drawn) at eight cameras from 25 m to 2.4 km and a generated town at seven were captured from the base commit and from the change with grass, effects and cast lights off: every frame is pixel-identical (0 of 2,073,600 pixels differ in each of 15 frames) and the model and scenery stats are equal. The decisions are in [choices](../choices.md#buildings-lane).

**Second half, 2026-10-01: a town is drawn from the template art library.** A building whose template the installed library has is its rows, as instances of its kit's modules; massing is deleted, and stand-in boxes come from the prototype set through the same rows. `/lab/city-block` is the route; `throwaway/evidence/city-block/` holds its frames after `scene -- city-block`.

- **The seam.** `BattleFrame.setBuildings({ placed, fallen })` (`models/buildingReferences.ts`): the map's references (template, frame, owner; 38 bytes a building) and the buildings the side has seen fall. The library and its kits arrive with `setAppearances`. The simulation's building export carries each building's `frame` for it. `presentation.buildings` in `fixtures/game.json` replaces `presentation.massing`: `lod_px_per_m`, `chunk_m`, `pool_records`, `expand_rows`, `tint_jitter`, `prototype_tints`, `ruin_tint`. `FrameStats.buildings` reports what a view draws, the pool, and what the last view change cost.
- **How it draws** (`models/buildingPlacements.ts`, `buildingLayer.ts`, `placementPool.ts`). Every building's rows at the coarsest tier are a population of the static chunk owner, expanded once for the map. A 64 m chunk near enough for a finer tier has its buildings' rows at that tier expanded once into a pool of 131,072 records, kept kind by kind (a module at a tier), and leaves it when it leaves the view or the tier; its coarse records are hidden in place meanwhile. A chunk the pool cannot take draws coarse. A kit module is drawn by the models layer's own material and pipelines, in the prepass's world half. The decisions and what each was chosen over are in [choices](../choices.md#buildings-lane).
- **Knowledge.** A building is intact until the side has seen any part of it fall. Then it leaves the intact rows and is drawn as its template's `ruin` rows where the library has them (none yet: C14), else each part as a box in `ruin_tint` at the size the side knows it. A change rewrites that building's coarse records and expands its chunk again; nothing else.

**Measured**, on a Metro Large (seed 1: 9,909 buildings, 22,340 coarse records in 3,827 chunks), in `/lab/city-block?type=metro&size=large&seed=1`, which draws the map with no battle on it, 1920 x 1080, development build, Apple Metal. The buildings' GPU time is the median of six paired differences (120 forced frames with them, 120 without, interleaved, `BUILDING_COST=1 CITY_MAP=metro:large:1 scene -- city-block`), with the range of the six. The machine's load average was 13 to 37 throughout, so read the frame totals as upper bounds.

| Camera on the main town | Buildings, GPU | Whole frame, GPU | Draws a pass (each cascade the same again) | Instances at tiers 0 / 1 / 2 / 3 | Triangles at tiers 0 / 1 / 2 / 3, and a cascade | Resident chunks, pool records | View change |
|---|---|---|---|---|---|---|---|
| Street, 25 m | 2.16 ms (1.75 to 2.35) | 6.17 ms | 197 | 1,678 / 3,005 / 7,078 / 5,213 | 355k / 229k / 460k / 480k, 819k | 160, 11,761 | 2.5 ms |
| Tactical, 65 m | 0.23 ms (0.05 to 0.68) | 3.21 ms | 91 | 1,708 / 0 / 0 / 10 | 364k / 0 / 0 / 3k, 135k | 5, 1,708 | 1.1 ms |
| 250 m | 1.08 ms (0.97 to 1.22) | 2.81 ms | 75 | 0 / 4,946 / 26 / 484 | 0 / 380k / 80k / 131k, 249k | 38, 4,972 | 0.9 ms |
| Whole map | 1.30 ms (0.93 to 1.52) | 9.80 ms | 19, no caster | 0 / 0 / 0 / 22,340 | 1,671k | 0, 0 | 0.9 ms |

- **Budgets.** The static city is under 15 ms of GPU at every camera tried; the whole-map view's 9.8 ms is the forest's, as in S3. The tier 3 instances at the near cameras include records hidden in place and records of chunks just out of view drawn to save a draw.
- **Memory.** In the generated battle on Mixed Small, against the same battle drawn as boxes: buffers 166.0 to 190.7 MiB (+24.7: the pool is 8.0, the rest the two kits' meshes at four tiers), textures 388.4 to 402.4 MiB (+14.0: 42 more 256 px layers). The coarse population of a Metro Large is 1.4 MiB.
- **Expansion.** A view change is one walk over the chunks and the expansion of those that entered: 0.9 to 2.5 ms at the stations, 3.5 ms for the largest seen (13,870 rows on a cut to the street, 1.0 MiB uploaded), about 0.25 microseconds a row. A pan across the town and a zoom from 25 m to 4 km peaked at 33,246 pool records with nothing refused. A still camera expands and uploads nothing.
- **Startup.** Building the references' scene took 24 ms for the Metro Large and 16 to 23 ms for Mixed Small (development build), beside installing the two kits.
- **The art's budgets.** No tier needed lowering: at the tactical camera the buildings draw 0.4 to 0.9 million tier 0 triangles for 0.2 ms. The thresholds are 10, 4 and 1.2 pixels a metre (tier 0 inside 128 m, tier 1 inside 319 m, tier 2 inside 1,064 m at 1080 pixels).

**Checked.** Unit tests of the pool, of expansion and residency, of masks at each tier, of a turned building against the resolver, and of knowledge (`placementPool.test.ts`, `buildingPlacements.test.ts`), each falsified by a mutation. The `city-block` scene: every building drawn from rows, an apartment block and a house where the map puts them and the right way round (the ground-classes view), the pool inside its bound over a pan and a zoom, a fall and its undoing, the buildings replaced and the frame rebuilt with buffers and bytes back to baseline, no validation warning. The `generated` and `camera` scenes pass on the new path. The fallen at eight cameras (the first half's capture) are pixel-identical before and after: 0 of 2,073,600 pixels differ in each frame, and the model statistics are equal.

**Pictures.** Against the same battle drawn as boxes, at the same cameras, 36% to 60% of the pixels differ at the tactical, 250 m and top-down views and 1.7% at the whole map: the change is the buildings. An unprimed critic found nothing floating, no z-fighting, no mirrored sign, no seam between tiers and shadows attached. What it did find, and whose it is:

- **Stand-in boxes are the brightest things in a town**, and a fallen building is a flat slab: the templates with no art yet (C16, C18, C19) and ruin art (C14).
- **Green strips inside some balcony recesses of the point block at tier 0**, which read as holes: for the China kit (C11) to look at in its reassembly sheet.
- **Distant balcony facades alias to a checker** at the far edge of the 250 m view, where the blocks draw their tier 2 shell: the kit's tier 2, or a later threshold.
- **Shadows inside fog read as a deeper fog**, stair-stepped shadow edges at 250 m, and the hold zone's ring showing as stray white arcs between buildings: as S3 recorded them; the fog look's (C20) and the encounter's.
- **Buildings that run into each other, entrances that open on grass, streets that end in grass:** the map's (parcels, C28).
- **Dark brick on walls facing away from the sun, a loud roof tile, repeated laundry:** the kits' materials.
