# 09 — Native model drawing architecture

*Rewritten after 08.* The anatomy found no remaining waiting once the harness drew its own frames (07). The cost is work: the pre-parity `legacy_map_layer.gd` draws each unit as an unbatched multi-part model and each prop at full detail, and every shadow cascade draws them again. With that layer off, a market-town frame drops from about 40 ms to 9.

**Contract:** native draws models the way the browser does, as a general architecture that slice 13's authored models will reuse:
- one instanced batch per appearance and detail tier, not one node per placed part;
- distance tiers and impostors chosen by the browser's rules;
- shadow casters culled to what each cascade sees.

Draw calls and primitives per frame scale with appearances × tiers, not with placed objects.

**Decision** (orchestrator, 2026-10-11): neither tune the throwaway layer nor measure around it. Build the lasting draw path now, on the content the legacy layer places today. Slice 13 then changes identity, materials and placement, not how models are drawn.

**Seam:**
- **Browser owners to mirror:**
  - `packages/battle-renderer/src/models/modelLayer.ts`: instancing and tiers;
  - `models/impostor.ts`: the impostor atlas and its pitches;
  - `apps/battle-lab/src/buildingTier.ts`: pixel-sized tiers;
  - `frame/` shadow caster selection.
- **Native:**
  - `world/model_batches.gd`: one MultiMesh per appearance part and tier, rebuilt only when membership changes;
  - per-frame updates write instance transforms only;
  - an impostor path for the far tier;
  - shadow casting limited per cascade (Godot cascade blend and caster culling, plus per-tier `cast_shadow`, so far tiers and impostors don't cast).
- **`legacy_map_layer.gd` shrinks to placement** (what stands where), and drawing moves to the batches. Units keep their published poses; articulation is slice 13's.

**Runnable artifact:** Godot's per-frame draw-call and primitive counts, which don't move with machine load, plus untraced frame ms, before and after, on market-town records 1000–1299 and the whole reel, in `throwaway/perf/batches/`.

**Focused gates:**
- **Draw calls per frame.** Market-town drops from about 20,000 to the low hundreds. Report the exact count and its formula: appearances × parts × tiers × passes.
- **Frame time.** The median untraced frame in the 1000–1299 window, minimised, three runs, is at or below the browser's market-town p50 (16.7 ms).
- **Plates.**
  - Batching alone is byte-identical within the noise floor.
  - Tiers and impostors may change distant pixels. Judge them against the browser control with `compare-screenshots`, and they must not move further from the browser. Then an unprimed `screenshot-critique`.
- `native/godot/tests/run.sh` stays green.

**Visual variable:** none intended. Distant models may move toward the browser's own impostors.

**Delegated:** MultiMesh grouping and the update strategy, provided the counts gate holds.

## Findings (2026-10-11)

Editor (debug) build, plugged in, M5 Pro, 1280×720, minimised, uncapped, 30 warm-up frames. Evidence in `throwaway/perf/batches/`. Frame figures come from the browser's summariser (`web/fps-compare.mjs`); counts are Godot's own and don't move with load.

**The cause was larger than 08 saw.** Every catalog model carries four authored tiers as `_LOD<n>` meshes, and the legacy layer drew all four on top of each other: four times the geometry, and the speckle where tiers z-fight. Units were scene instances (an Abrams is 1,247 mesh nodes), drawn node by node in each of six passes.

**Market-town records 1000–1299, three runs each:**

| | Draw calls | Primitives | Frame p50 (runs) | p95 | p99 | CPU p50 |
| --- | --- | --- | --- | --- | --- | --- |
| Before (`3d22b06a3`) | 19,922 | 18.0 M | 34.75 ms (32.07–35.53) | 39.27 | 50.66 | 15.4 |
| Batched, every tier drawn (`355be5442`, one run) | 650 | 230 M | 49.5 ms | 54.1 | | 1.3 |
| After | 410 | 4.24 M | **8.45 ms** (8.44–8.47) | 9.10 | 14.01 | 1.1 |

Gate: 8.45 ms against the browser's 16.7 ms p50. Batching alone cut draws 30× but raised primitives, because a MultiMesh spanning the map is culled whole and Godot's own LODs were off; tiers and chunk culling brought them below the legacy-free ground of 4.0 M.

**The formula.** Draws = 2 passes (depth prepass, colour) × the view batches' surfaces + each caster batch's surfaces × the cascades it reaches + 8 for ground, grass, sky and post. On record 1100: 160 view surfaces (units 141 over 9 appearance-tiers, buildings 14, props 5) give 320, caster batches 80 surface-cascade draws, ground 8: 408, as Godot counts. A view surface is one material of one appearance at one tier, so draws scale with appearances × tiers × materials, not placements.

**Whole market-town reel, one run** (60 warm-up): p50 10.34 ms, p95 26.09, p99 30.61, max 46.37; 1,068 draws and 16.0 M primitives a frame on average (max 1,995 and 33.0 M). Before: 75 ms median, 20,023 draws, 34.2 M primitives. The heavy frames are the wide city views: every kit shell draws all four tiers, and far props draw their coarsest tier out to the horizon. The browser's market-town p50 is 16.67 ms (p95 25.01).

**Plates.**
- Godot's automatic LODs off, old code (`nolod`): 66–3,238 pixels a plate move, all on distant models, which now draw the authored mesh.
- Batching alone against that: 4–180 pixels over 2 levels on market-town plates, 4–118 on paris-corner (rerun floor: 0–1). Every one is on the overlapping tiers' z-fighting or shadow speckle, which batching reorders, plus sub-level vertex differences: merged meshes keep full-precision positions where Godot's import quantised them. Not byte-identical; no visible change.
- Tiers and casters against the browser control (`compare-screenshots`, mean parity distance over each scene's plates): market-town 0.0648 → 0.0641, paris-corner 0.3145 → 0.3135. One plate moved away by more than 0.002 (market-town 6402, 0.0498 → 0.0533): the street trees' coarser crowns. The changes are silhouettes: one tier where four overlapped.
- Unprimed `screenshot-critique` (browser, after and before, as neutral A/B/C, five plates): after and before are visually the same on every plate; after is "very slightly cleaner" on market-town 8278 (no tier speckle at a block's base). The gaps it found are shared and already slice 13's: few and T-posed soldiers, untextured cars, simplified buildings, larger trees. It also saw that soldiers cast no shadow beside their contact disc, and that the paris-corner tank reads as floating (the browser's dust hides its contact there); both are in the legacy look and go to slice 13.
- The first tier pass drew kit shells at one tier, which left Paris blocks hollow (a shell's finer tiers have openings for facade modules the legacy layer doesn't place); shells now draw every tier together.

**Not done.**
- **Impostor cards.** Deferred to slice 13 (see `choices.md`): the browser's cards are for posed soldiers, native's are unposed, and the frame has 8 ms of margin.
- **Byte identity of the batching step,** for the reasons above.
