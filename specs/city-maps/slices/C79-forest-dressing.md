# C79: forest dressing

**Depends on:** C76, C78, SG1. **Kind:** slice.

## Question
Is the forest floor full of ferns, bushes, saplings, small rocks and litter within budget, without false cover?

## Contract it unlocks
- A third population in `scenery/placement.ts`, **drawn by the existing scenery layer's chunks** (`frame/sceneryLayer.ts`, `scenery/lod.ts`), not C22.
- Seeded per forest, kept off trunks, bodies, roads and cleared lanes. Everything ≤0.9 m; rocks ≤0.5 m.
- Far chunks leave it out, and it fades by pixel size within GG's radius. Presentation-only.

## API seam
`scenery/placement.ts`, `sceneryLayer.ts`, the biome's `forest_floor.dressing`.

## What the human can run or see
`forest-deep-25` and `forest-edge-65`, crowns off, then on.

## Verification
- Instance count and bytes; a frame-cost row; benchmark worst window.
- A test that nothing is placed outside forests.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**the forest floor below 2 m; critique: "does any bush look like it hides more than the forest does?" and the shadow question**) against **`../assets/reference/ground/forest-road-summer.jpg`, `forest-road-rocks.jpg` and C78's shot**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Canopy.


## Delegated to the implementer
Mix and densities (density is the cut knob). Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
C76's frame budget; the forest-floor fog.

## Feedback that would change this slice
Dressing that spills outside real forests changes distribution; a performance failure follows the stated density/cut order.

## Outcome (2026-10-02)

**Contract as landed.**

- **The kinds.** Five `dressing` appearances from `forest_floor.py`: `floor_fern`, `floor_bush`, `floor_sapling`, `floor_rock`, `floor_litter` (fallen branches). `dressing` is a scenery kind with no body: the validator holds its art under 0.9 m (`fit.dressing`) and its tiers under 600 / 200 / 60 / 24 triangles, and the biome may not scale a piece above its art. The tallest is the sapling at 0.86 m; the rock is 0.33 m.
- **The rows.** `forest_floor.dressing` in the biome: `per_ha` (the cut knob), the kinds with their weight, drift, scale and tint, `drift_m`, `squat`, `edge_m`, `trunk_clear_m`, `clear_m`, `colour_jitter`, `lod_px`, `fade_px`.
- **Placement** (`scenery/placement.ts`, `DressingField`). The ground is dressed a 64 m cell at a time, seeded by the cell: `per_ha` candidate points, each a kind by weight, thinned between that kind's drifts (a simplex field per kind), kept only 3 m inside forest ground and clear of trunks (0.9 m), bodies, paving and water (1 m). A cell is the same whenever it is laid.
- **Residency** (`scenery/dressing.ts`). Only the cells a camera can see pieces in are laid, into one instance pool of at most 128 cells, two a view and nearest first; the cell wanted longest ago gives up its slot. `stats().scenery.dressing.pending` says cells are still to come, and the rig's `shoot` waits for it.
- **Drawing** (`frame/sceneryLayer.ts`). A cell draws whole at one tier from its slot, in the depth prepass and the world pass, through the forest's own fragment stage (shadowed; fogged whole, like a tree). No cascade draws it. Each piece shrinks about its foot in the vertex stage as its projected height falls from `fade_px` (6) to half of it, and a cell whose tallest piece would be gone is neither drawn nor laid. Ground the side has seen cleared holds none.
- **The switch.** `BattleFrame.setDressingShown`, the lab's `suppressDressing`. `trees: false` in the rig leaves the dressing drawn.
- **The checks.** `web/scenes/_forestFloor.mjs` in the `ground` scene (`FLOOR_ONLY=1` alone; `FLOOR_SHOTS=1`, `FLOOR_COST=1`): the pixels the dressing changes at the wood's edge all lie on forest floor. `web/tests/scenery.test.ts` and `web/tests/dressing.test.ts` hold the placement rules and the cache.

**What changed from the slice as written.** The dressing is not a third list in `placement.ts` expanded for the map and bucketed by the static chunk owner, and it is seeded by cell, not by forest: the saved small generated map would hold 657,000 pieces (31.5 MB, 0.42 s at load) that way. It shares the layer's record, meshes, vertex placement and fragment stage, and has a cache of its own in place of the chunk owner. GG has given no radius: the fade is by pixel size alone.

| Measure | Value |
|---|---|
| Candidates a hectare / share that stand | 3,200 / about six in ten |
| Pool | 1,311 records a cell; 2.0 MB on the village (32 cells), 8.1 MB at most (128 cells) |
| Laying a cell (CPU) | 0.25 to 0.5 ms, 4.5 ms the worst seen; two a view |
| Choosing cells, view settled (CPU) | 1 to 4 µs |
| GPU, village `forest-65`, on against off | **+0.25 ms** (four pairs: 0.25, 0.18, 0.32, 0.19; 4,336 pieces, 282,000 triangles; bare frame 1.98 ms) |
| GPU, generated `forest-edge-250` | **+0.08 ms** (0.08, 0.10, 0.03, 0.00; 4,511 pieces at the far tier, 83,000 triangles) |
| Scenery draw calls a frame | 72 to 132 (village), 84 to 196 (generated) |
| Dressing pixels at `forest-edge-65`, crowns off | 18,433 of 1,035,794 floor pixels (1.8%); 0 on the verge, 0 outside |
| Floor seen through the crowns, 65 m / 120 m | 0.231 / 0.220 (C76: 0.23 / 0.22) |

Apple Metal, 1920×1080, 120 forced frames a batch. The first build drew every near piece a tier finer and cost +0.79 ms at `forest-65`; `lod_px` was coarsened to `[48, 22, 8]`. Not run: the benchmark's worst window.

**Compare** (before against after, same stations). With the crowns off at 25 m the bare floor's edge density goes from 0 to 0.05 and its mean luminance from 90.3 to 90.0 of 255: the dressing breaks the floor up without darkening it. With the crowns on at 65 m the frame moves by 0.05 to 0.13% of its pixels: the canopy hides nearly all of it. Against `forest-road-summer.jpg`: theirs is a continuous, muted carpet of fern and litter; ours is separate pieces on a smooth floor.

**Critique** (unprimed, twice; the kinds, colours and rules were changed once between). First: ferns read as "tiny palm trees" in "saturated lime-yellow", "obvious clones", pieces "on the forest's border over the field", painted root arcs "more prominent than the real sticks". Done about it: narrower, notched, drooping fronds in a muted green; a wider size range and a build that varies; `edge_m`; thicker branches; fainter roots (`forest_floor.roots` 0.25 to 0.1); smaller bushes. Second, on the final models:

- No single piece looks able to hide a soldier (the slice's own question): "the largest bush is well under half the boulder's width and looks knee-high".
- Ferns still read as "palm crowns or spider plants" close up and as dark asterisks at 65 m; bushes as "clusters of smooth balls"; saplings as "broccoli sprigs". Left: models are good enough.
- Bushes were the brightest thing in the wood: their tint was muted like the ferns' after this critique and shot once more. In a sunlit gap they are still brighter than the ferns beside them. Not critiqued again.
- Nothing casts a shadow, so pieces look "pasted on". Left: four cascades of dressing.
- Density is "a thin even sprinkle on a mostly bare floor", and from the play camera under crowns "only a handful of specks"; at 250 m nothing. So the slice's question is answered **no, the floor is not full**: it is dressed, sparsely. Density has 0.75 ms of GPU to spend and costs pool memory in proportion.
- "Less wrong than bare brown at the close camera, about the same at the play cameras."
- Not this slice's: the floor's smooth airbrushed ground and its dark drifts (they "look like crown or cloud shadow" with the crowns off); the scalloped verge over the field, which reads as "a spilled-paint mask".

**GPU.** Seven holds of the lock for C78 and C79 together, against the budget's five or so: the before set; a first village set; two densities; the candidate's full set with its checks and cost; the final set after the critique with the ground scene and the cost again; the floor check alone after its bar was restated; the village once more for the bushes' tint.

**Pictures** (scratch): `throwaway/shots/final/` against `throwaway/before/`; the first build in `throwaway/shots/after/`.
