# C63: surface distance field

**Depends on:** C03, SG5. **Kind:** slice.

## Question
Do terrain and grass read every ground rule (road, forest, river) from one exact-enough distance structure, with the per-fragment loops gone?

## Contract it unlocks
- **Owner:** `terrain/surfaceField.ts`, built from C03's export with SG5's chosen structure (a signed-distance bake or a segment-bucket index). It uses **the same distance function as the sim**.
- It is a signed distance, **never a class or coverage mask**.
- `groundSite` and `groundWater` sample it, and the loops at `terrainMaterial.ts:236-243, 371-380, 385-394` are deleted. Grass reads it through `groundSite`.
- **C28 is reordered after this slice** and reads this field (its pavement bake becomes a consumer, not a second owner).

## API seam
`packages/battle-renderer/src/terrain/surfaceField.ts`, `terrainMaterial.ts`, `grassPass.ts`.

## What the human can run or see
The village, pixel-equivalent before and after.

## Verification
- Village terrain and grass frames compared before and after with compare-screenshots (must be unchanged within tolerance).
- Export-to-field agreement within SG5's ratified error at curated edges/joins/width changes and boundaries, using the shared geometry owner as oracle.
- Bounded resident/peak bytes and update/upload costs at each selected extent, full overview and opposite-edge pan; no full dense field is assumed affordable.
- Bytes; a frame-cost row.
- Digests untouched.

## Delegated to the implementer
Resolution and packing within SG5's verdict. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
The village look; the `fog-look` and ground scenes.

## Feedback that would change this slice
Disagreement between physical boundaries and the rendered field reopens export/distance parity before consumers tune art.

## Outcome

**What was built.** `terrain/surfaceField.ts` owns one structure for every ground rule: a table of the simulation's exported primitives (paved strokes, triangles and exposed boundary edges; each forest's strokes, triangles and edges; forest rects; water rects) and a bucket index over them. The terrain and the grass look up the cell under a point and loop only that cell's list, with the distance functions they had before. The three all-primitive loops are gone. It is a signed distance throughout; nothing is baked and there is no mask.

**The seam.**
- `groundCell(xy, footprint)` returns where the point's paved, forest and water lists start and end. `groundSite(xy, cell)`, `groundWater(xy, cell)` and `groundDapple(xy, footprint, cell)` take it. `groundSurface` is gone; the world pass does one lookup per fragment and shares it.
- The bind group has the same four storage buffers: the rect table became part of the records, and `surfaceIndex` (u32) took its binding. `TerrainParams` swapped its three count fields for `field` (grid origin, 1 / cell, the finest level's pixel width) and `fieldGrid` (cells across and up, levels).
- `groundReach(biome, footprint)` says how far each rule is read, reader by reader, from the biome's numbers. The export layout and the Rust side did not change; digests are untouched.

**Reach, and why it is a ladder.** Every reader feathers over a pixel at least, so the reach grows with the pixel: paved `0.9 + footprint` m (the verge), forest `4.5 + footprint` m (the floor's ragged verge), water `max(3, footprint / 2)` m with the summer biome (each with a floor of a metre or two for the grass margin). One grid cannot serve a 0.08 m pixel and a 200 m one, so the index is a ladder: level 0 has 8 m cells and serves pixels to 2 m, and each level above serves twice the pixel with cells at least four of its footprints a side. Within its level's reach a lookup equals the all-primitives distance exactly; beyond it the value keeps its side. On a sparse map the ladder ends in one cell listing everything, so any pixel reads exactly. On a dense map it stops at a list budget (`choices.md`).

**Cell size, measured** (bytes, and the mean records a lookup visits at a play-camera pixel):

| Ground | Records | Finest cells | Levels | Index | Records | Visited |
|---|---|---|---|---|---|---|
| Village (with C65's rounded bends) | 103 | 200 × 200 at 8 m | 9, exact at any pixel | 651 KiB | 3 KiB | 0.1 |
| Geometry lab | 3 | 50 × 38 at 8 m | 7, exact at any pixel | 32 KiB | 96 B | 0.2 |
| Synthetic town, 1.6 km | 4,342 | 201 × 200 at 8 m | 5, exact to 32 m pixels | 1,019 KiB | 136 KiB | 1.1 |
| Synthetic city, 6 km | 25,902 | 375 × 375 at 16 m | 6, exact to 64 m | 5.7 MiB | 0.8 MiB | 0.8 |
| Synthetic city, 10 km | 51,702 | 313 × 313 at 32 m | 6, exact to 64 m | 6.8 MiB | 1.6 MiB | 1.3 |

The old loops visited every record at every pixel. At 16 m cells the town visits 1.85 records for 568 KiB; cells finer than 8 m gain nothing, because a level-0 cell is four of its 2 m footprints. The finest level never holds more than 262,144 cells: a larger map doubles its cell. The build is 10 to 40 ms for the village and the town and 120 to 600 ms for the 10 km city on a loaded machine, once per world.

**Pixels.** Ten village cameras (default, road junction and bend, forest corner and edge, mid, full zoom-out, three low grazing views) and three geometry-lab cameras (road, forest, water), each captured twice at a pinned tick with the clock held. Compared twice: against the slice's starting commit (the five-stroke village), and, after merging C65, against that tip (the village with rounded bends). Both times:
- terrain alone (grass, models, fog, effects and paint off): 26 of 26 captures byte-identical, 0 differing pixels;
- the grass build's clump readback (position, height, kind, tier, lay): identical at all 13 cameras (240,369 clumps on the first comparison);
- frames with grass drawn differ by 68 to 2,201 pixels, the same as two runs of an unchanged build (68 to 2,231): Metal's grass depth ties, not the field.

**Agreement.** The CPU lookup equals the all-primitives distance at about 190,000 seeded point-and-pixel pairs over a curated ground (joins, a width change, overlapping and touching paving, concave, strip, rect and overlapping forests, water), the village, the geometry lab and the synthetic town. The GPU lookup equals the CPU one at 60,000 values on each of the curated ground and the town, worst 0.07 mm, in the ground scene.

**Frame cost.** Paired on the GPU: one 1920 × 1080 frame's worth of points through the old loops and through the field, interleaved, median of 25 pairs (Apple Metal; machine load average 60 to 90, so the absolute numbers are high and the pairing is what holds):

| Ground and view | Old loops | Field |
|---|---|---|
| Village, 103 records, default-camera window | 4.2 ms | 0.7 ms |
| Village, whole map | 4.3 ms | 0.8 ms |
| Town, 4,282 records, default-camera window | 159 ms | 1.1 ms |
| Town, 4,282 records, whole map | 142 ms | 1.6 ms |
| Town, 12,762 records, default-camera window | 383 ms | 1.2 ms |
| Town, every point a 40 m pixel (worst case, grazing) | 143 ms | 9.5 ms |

Before C65 the village was 7 records and the same measure read 0.78 ms against 0.40 ms (load average 45): C65's rounded bends made the old loops five times dearer, and the field takes that back. The last row is every pixel at the budget's level; in a real ground view about 3% of the frame is that wide.

The benchmark's whole-frame row could not separate the two builds on this machine: with other agents running, the same build read 6 to 17 ms of GPU. The runs, for the record (GPU mean, load average): before 12.5 ms (40) and 16.8 ms (70); after 6.8 ms (25), 6.1 ms (35) and 12.4 ms (65). Buffers 182.1 MiB before, 182.7 after (the index), 400 buffers both; textures unchanged. It needs a rerun on a quiet machine for a `frame-cost.md` row.

**Scenes.** Ground, geometry, foundation, fog-look, workbench and village pass. On the merged tree the village scene took three runs under load: one passed 103 checks and hit the 900 s limit, one failed its two reset checks because a pause landed a tick late (tick 61 against 60), one passed all 121.

**Still open.**
- Mixed stroke and polygon joins are unchanged (`choices.md`).
- The list budget's look on a dense map at grazing angles has no generated map to be judged on yet.
- A corner case found on the way: `polygon2.intersectsSegment` misses a segment through a box's opposite corners, which dropped 45° strips from their cells. The builder clips to slabs instead, with a test.
