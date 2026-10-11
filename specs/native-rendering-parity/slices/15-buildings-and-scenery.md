# 15 — Buildings, scenery and their state

**Contract:**
- Native draws the browser's buildings module by module, facades included, and its scenery: forest trees, understorey, backdrop and dressing.
- Both use the tiers the browser chose.
- Fallen buildings, wrecks, felled trees and cleared trees come from the publication.

**Seam:**
- **Placement is static per map, so the browser exports it.** The manifest gains hashed artifacts, built by the same owners the browser draws from:
  - building module instances per template and state (intact, ruin, gutted): `templateLibrary.ts`, `buildingPlacements.ts`, `buildingReferences.ts`;
  - scenery instances: `scenery/placement.ts`, `dressing.ts`, `forestShapes.ts`;
  - the tier rules: `buildingTier.ts`, `scenery/lod.ts`.

  Native ports none of the placement logic.
- **State is per frame, so native derives it from the publication** (slice 13):
  - known destroyed buildings swap state (`destroyedBuildings.ts`, `setFallenBuildings`);
  - `knownProps` wrecks and shoved bodies;
  - `fallenBodies` fall at `(tick−1)/tickHz` (`scenery/felled.ts`);
  - `cleared` foliage.
- Draw through `model_batches.gd`. `legacy_map_layer.gd` is deleted, and the token-matched shells, the 1 km reach and the `street_tree` cone go with it.

**Runnable artifact:** plate pairs: paris-corner close cuts for balconies, railings and lamp posts, and market-town broad cuts for tree lines and forests.

**Focused gates:**
- **Counts.** Per plate, native's drawn building-module and scenery instances per kind equal the browser's, which the browser exports from its layers in the plate's frame.
- **Visual.** `compare-screenshots`, then an unprimed `screenshot-critique`, then `preview-shots`.
- **Cost.** This layer's pass cost, and slice 12's measurement re-run on both scenes, with native at or below the browser.

**Visual variable:** buildings and scenery: identity, completeness, placement and state.

**Delegated:** artifact packing.

## Findings (2026-10-11)

Control `throwaway/native-parity/2026-10-11T12-59-45-367Z` (browser half `3f14bf2c9`, props export `40425bdbc`). Native `e41b00362`, `ce1356947`. Evidence in `throwaway/slice15/` (pairs, critique, comparisons) and `throwaway/perf/slice15/`.

**Counts: pass.** Native's instance counts equal the browser's `instanceCounts` on all 24 plates (6 paris-corner, 18 market-town): every kit module per tier, every scenery population per appearance and tier (felled trees and stumps included, up to 14 by the end of the reel), and every known prop per appearance and tier (the T-72 and Abrams wrecks, a car wreck, the shoved parked car). Any difference is an `instances` blocker. No building falls in either reel, so the fallen path is proven only by `building_layer_test.gd`.

**Visual: pass with two notes.** `compare-screenshots` mean distance over each scene's plates: market-town 0.021 (slice 09's legacy look: 0.064), paris-corner 0.159, dominated by the browser's effects (an explosion fills plate 1199; smoke on 1698), which are slice 17's. The unprimed critique found nothing missing, extra, misplaced or in the wrong state, on both scenes. Its one finding, wrought-iron railings losing their bars at distance, is fixed: a cutout's coverage rides its normal map's alpha, scaled and dithered as the browser's `cutoutCoverage`. Two differences are left. Distant railings dither by pixel where the browser dithers by MSAA sample, so they read slightly stippled. A street lamp's diffuser is grey where the browser's is white (emissive, not ported).

**Cost: fails on market-town.** Release export against the browser's production build, one interleaved round (`throwaway/perf/slice15/timing/`):

| scene | client | p50 | p95 | p99 | avg FPS |
| --- | --- | --- | --- | --- | --- |
| market-town | browser | 15.90 | 18.18 | 25.22 | 74.1 |
| market-town | native | 22.79 | 35.90 | 38.53 | 43.2 |
| paris-corner | browser | 8.60 | 17.26 | 18.22 | 90.1 |
| paris-corner | native | 8.47 | 10.03 | 16.66 | 116.1 |

Market-town records 1000–1299 (release, `bench.sh`): with the world's layers off, the frame is 8.4 ms with 1.5 ms of CPU and 4.2 M primitives. With them, before the fixes, it was 29.7 ms with 14.9 ms of CPU and 29.9 M primitives. After, it is 24.0 ms with 9.4 ms of CPU and 29.5 M primitives.
- **CPU** was the layers' GDScript. Each fix and what it saved:
  - re-tier a chunk model by model only when its nearest and farthest can disagree, and rewrite a chunk's packs only when a tier changed: about 6 ms;
  - index the forest by trunk id once, not every frame: 5 ms;
  - cull chunks by cell (`chunk_grid.gd`): 1 ms;
  - write batches only when the plan changed.
  
  What is left is the three layers' chunk walks: buildings 1.4 ms, scenery 2.2 ms, props 2.9 ms. Godot's frame is serial, so CPU adds to the frame.
- **GPU.** Ablation by layer: props cost about 10 ms, scenery about 4 and buildings about 3. Props are 5.1 M triangles in view at frame 1200 and about 15 M in the cascades: 9,105 parked cars at their coarsest tier (300 triangles) and 15,600 wall modules. That follows the browser's own rule: every static model in view is drawn, and drawn again into every cascade. Native spends about 0.55 ms per million primitives, which looks slower per primitive than the browser's 9.0 ms GPU for its frame. Re-uploading every buffer each frame costs nothing measurable.
- **Not tried yet:** casters batched by region so Godot culls them per cascade (props' cascades are about 3 ms), moving tier choice for statics into a compute pass as the grass does, and cutting the remaining chunk walks.

**Not ported:** leaf clumps, the dressing's shrink below a few pixels, wear chips, lit interiors, emissive surfaces. Each is reported as an approximation. Cook-off transitions (a wreck drawn moving while it cooks off) are not ported either; no plate falls inside one.
