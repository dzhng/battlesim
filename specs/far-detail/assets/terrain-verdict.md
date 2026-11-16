# Terrain verdict

Would a cluster hierarchy, coarser far from the camera, save GPU time worth
owning? Rung 1 of [slice 06](../slices/06-spike-terrain.md), the count, recorded
2026-11-15 on b0a60c7a3. No timing was run.

## Verdict: no-go at rung 1

Terrain is at most **2.6% of the triangles a frame submits**, on the hilliest
map a player can get, in the worst wide view of it. At the whole-map view it is
1.3%. Props, scenery and buildings submit the other 97 to 99.9%. Even a terrain
that cost nothing would remove at most a fortieth of the frame's triangles. The
earlier timings point the same way: terrain shadows cost about 0.4 ms at the
wide hold, and choosing terrain ranges by view saved 0.22 ms. Rung 2 is not
worth its GPU session. The spike ends here.

One part of the slice's rule does not hold, and the verdict does not rest on
it. The rule reads "a small share **and** its triangles are a pixel or larger".
The share is small. But carved ground (river banks and swamps) is drawn at
the simulation's 4 m grid, so at the wide holds those triangles are about a
third of a pixel each, and at the whole-map view about 120 share a pixel. So
terrain does carry sub-pixel geometry. There is just too little of it to matter:
at most 581,000 terrain triangles in a frame of 22 million at the wide holds,
and 1.6 million of 124 million at the whole-map view.

## Which map is the hilliest a player can get

Players fight only on generated maps. No saved map is a player's
([fixtures README](../../../fixtures/README.md): "player battles are generated
and never name one"). The skirmish choices are a type (open, mixed, metro), a
size (small to XL) and a seed (`web/src/maps/source.ts`).

Generated land has no relief of its own: the generator writes an empty relief
list (`crates/mapgen/src/lib.rs`, and "Generated land has no relief" in
`open_country/coverage.rs`). The only height on a player's map is carving:
- **River.** A river channel cuts down to its depth, with banks graded back
  to the land (`crates/sim/src/world/carve.rs`).
- **Swamps.** A swamp lowers the land by 0.6 m (`map-presets.json`, `swamps.depth_m`).

So every generated map is flat land at height 0 with carved channels, and none
goes above 0. The terrain mesh keeps the 4 m grid wherever the ground is
carved and merges every all-zero rectangle into large flat triangles
(`crates/sim/src/world/terrain.rs`). A map's terrain triangle count is
therefore set by how much ground is carved. A map with no river and no swamp
is **2 triangles**.

I generated 72 XL maps the way a battle does (`generate_map` with the game
catalog's rules, then the simulation's own terrain export): open seeds 1–60 and
mixed seeds 1–12. Metro generates too slowly to scan (about 2.5 min per map),
and its rules give it the fewest swamps (chance 0.2 against open's 0.45). XL is
the largest size, 10 × 10 km. Across the 72:

| | Open XL (60) | Mixed XL (12) |
| --- | ---: | ---: |
| Terrain triangles, min / median / max | 2 / 40,442 / 255,456 | 2 / 104,569 / 260,456 |
| Maps with a river | 28 | 6 |
| Deepest point | −9.65 m | −9.38 m |
| Highest point | 0 m | 0 m |

No map had more than one river, and a river is about 200,000 triangles of the
total; swamps add 10,000 to 20,000. **The hilliest map counted is mixed XL seed
12**: one river, 260,456 terrain triangles, 82,297 of them carved, deepest
point −8.31 m. The deepest map is open XL seed 8, at −9.65 m, with 222,986
triangles. The receipt's city, Metro XL seed 4, has a river and two swamps:
216,958 triangles, deepest −9.47 m. That is within 20% of the most found, so
the hilliest map adds little terrain beyond the city map slice 00 counted.

## The counts

State: the view-cost state slice 00 used. The `city-arena-2` stress recipe
fields 100 a side, paused at tick 90, Low graphics, on a 1280×720 page with a
960×540 canvas, yaw −1.57, FOV 0.8. The hilly map runs the same recipe; the
recipe fits any map of at least 3 × 2 km. Views:
- **Protocol wide.** 2,400 m over the densest own soldier, as slice 00's.
- **Worst wide.** 2,400 m over the densest 2.5 km window of carved triangles.
- **Whole-map.** The map centre at the player's farthest zoom, 19,500 m
  (1.5 × the rendered span, `gameCamera.forMap`), at its overview pitch 1.3.

Exact cameras, identities and every number below are in the
[receipt](terrain-count.json).

Submitted triangles are the renderer's own counter (`stats().submitted`,
`crates/renderer-core/src/submitted.rs`). "View" is the colour pass; the depth
prepass submits the same terrain. Share is terrain's part of everything the
frame submits, across depth, colour and the four cascades.

| Map | View | Terrain, view pass | Terrain, cascades 0 / 1 / 2 / 3 | Terrain share of the frame |
| --- | --- | ---: | --- | ---: |
| Metro XL 4 (city) | protocol wide (4864, 4852) | 10 | 10 / 10 / 16 / 2,835 | 0.003% of 105.5 M |
| | worst wide (7500, 4250) | 75,224 | 84,251 / 96,416 / 100,414 / 110,187 | 2.49% of 21.7 M |
| | whole-map | 216,958 | 216,958 each | 0.85% of 153.7 M |
| Mixed XL 12 (hilliest) | protocol wide (4859, 4429) | 51,956 | 42,568 / 64,820 / 100,327 / 109,821 | 1.28% of 32.9 M |
| | worst wide (4000, 3000) | 78,133 | 94,633 / 104,664 / 110,631 / 114,645 | 2.61% of 22.2 M |
| | whole-map | 260,456 | 260,456 each | 1.26% of 124.2 M |

The city's protocol row reproduces slice 00 exactly: 10 to the view and 2,871
to the cascades, at digest `ef325ab1cc76c583`. The hilly map's digest at tick
90 is `23b4008cda9c063a`. At the whole-map view every pass draws the whole
terrain mesh in one range. Props and scenery still dominate there: scenery is
98 million of that frame's 124 million triangles on the hilly map, and 87
million of 154 million on the city.

## Triangles per covered pixel

Every terrain triangle was projected with the camera each view drew with
(`camera3d`'s view-projection) and clipped to the canvas. Occlusion is
ignored; a height field seen from above barely overlaps itself.

| Map | View | Terrain on screen: triangles / pixels | Carved: triangles / pixels | Carved px per triangle, median |
| --- | --- | --- | --- | ---: |
| Metro XL 4 | protocol wide | 7 / 518,400 (whole canvas) | none | — |
| | worst wide | 63,617 / 517,252 | 24,387 / 11,436 | 0.36 |
| | whole-map | 216,958 / 104,322 | 82,101 / 682 | 0.008 |
| Mixed XL 12 | protocol wide | 31,940 / 518,400 | 9,720 / 2,049 | 0.20 |
| | worst wide | 63,220 / 518,400 | 20,084 / 8,009 | 0.33 |
| | whole-map | 260,456 / 104,322 | 82,297 / 696 | 0.009 |

The flat ground beside a channel keeps fine triangles too, because the merge
takes only whole all-zero rectangles. Elsewhere one flat triangle can cover
most of the screen. At the wide holds carved ground covers 0.4% to 2.2% of the
canvas; at the whole-map view, under 0.7% of what the map covers on screen.

## What would reopen it

- A generator that raises land: authored or generated hills, so relief
  stops being a few river strips. Terrain triangles would then grow with the
  map's area rather than its rivers, and this count should be repeated.
- The feedback the slice names, "big maps over this one view": the
  `geometry-scaling` scene's synthetic terrain. That is not a map a player can
  reach today.

Rung 2, if it were scheduled, would draw the terrain from a coarser copy (a
quarter, then a sixteenth) under [the protocol](../README.md#measuring), on
the hilly map's worst wide view, with a quiet GPU. It competes with slice
03's timing session. On these counts its prize is at most the GPU time of
2.6% of the frame's triangles, so I do not propose it.

## Evidence and method

- Scratch, never committed, deleted with the checkout:
  - the map scan (`terrain-scan.mjs`: `generate_map` and `WorldView` from the
    shipped WebAssembly);
  - the hold driver (`terrain-hold.mjs`: the endurance route's view-cost
    hooks, with the route reading the map's type, size and seed from its
    address for this run only, reverted after);
  - the projection (`terrain-pixels.mjs`);
  - logs and per-view JSON, under `throwaway/far-detail/` in the slice 01
    checkout.
- The battle's map hash differs from the scan's for the same seed, because
  the battle names its entries per side and the scan did not. The terrain is
  the same: the whole-map submitted totals equal the scan's mesh counts
  exactly.
- No renderer or presentation code was changed or added: every count is
  from existing counters.
