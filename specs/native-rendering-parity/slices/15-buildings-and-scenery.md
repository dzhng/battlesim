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

**Cost: fails on market-town.** Release export, whole reel, control `2026-10-11T15-33-53-097Z` (effects drawn), against the browser's production build (`throwaway/perf/slice15/timing/`):

| scene | client | p50 | p95 | p99 |
| --- | --- | --- | --- | --- |
| market-town | browser | 15.90 | 18.18 | 25.22 |
| market-town | native, before the levers | 22.79 | 35.90 | 38.53 |
| market-town | native | 18.09 | 24.61 | 25.60 |
| paris-corner | browser | 8.60 | 17.26 | 18.22 |
| paris-corner | native | 8.60 | 9.96 | 18.36 |

The levers, measured on market-town records 1000–1299 unless noted (p50 frame, CPU p50):
- **Threads.** `thread_model=2`: no gain. Three frames in flight: 24.2 → 23.7 ms. Kept.
- **Depth prepass.** Off: 26.9 ms, and 31.8 against 22.8 late in the reel. Kept on: the frame is fragment-bound.
- **Casters by region.** In GDScript the bucketing cost more CPU than it saved. In Rust, 512 m regions: 23.6 → 18.6 ms (draws 2,638, primitives 29.5 → 16.4 M); 256 m 18.6, 1,024 m 19.6.
- **The walk in Rust** (`StaticLayer`): CPU 9.7 → 6.4 ms, with every static batch identical to the GDScript's. Felled trees moved to the vertex stage, which removed 10–15 ms of CPU bake late in the reel.

What is left is late in the reel (records 5,000 on), at 21–25 ms a frame, GPU-bound with 8–12 ms of CPU. On records 7,500–7,699: shadows off saves 2.2 ms; props off 6.2 ms (7.4 M of 16.5 M primitives: 7,171 parked cars and 10,521 courtyard walls at their coarsest tier, in view and cast, as the browser's rule draws them); scenery off 2.5 ms. With every world layer off the calm window is 8.6 ms (terrain, grass, sky, haze, post), the display interval. Native draws the browser's instance set, so the remaining gap is Godot's cost per primitive at 4× MSAA with a separate depth prepass, not the layers' choice. Closing it needs a renderer decision (slice 18).

**Not ported:** leaf clumps, the dressing's shrink below a few pixels, wear chips, lit interiors, emissive surfaces. Each is reported as an approximation. Cook-off transitions (a wreck drawn moving while it cooks off) are not ported either; no plate falls inside one.
