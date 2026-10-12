# 20b — The static world resolved in Rust

**Contract:** what the world holds before anything happens is resolved once, in Rust, for both clients:
- building template expansion into kit modules and states;
- scenery placement (forest, understorey, backdrop, dressing);
- the terrain tables (height pages, plots, surfaces, params);
- the grass tables (props grid, shapes);
- the tier rules.

The browser draws Rust's output; native draws the same output with no browser export in between. The TypeScript resolvers and the manifest artifacts that carried their output to native are deleted.

**Seam:**
- The resolvers move next to the map and catalog owners in Rust: `mapgen` and `contract`, or a new crate. The research step decides where.
- The output is flat records both renderers consume as GPU instance or table data.
- The browser gets them through WASM, replacing `templateLibrary.ts`, `buildingPlacements.ts`, `scenery/placement.ts`, `dressing.ts` and the terrain and grass packers.
- The manifest keeps identity, lens, light and frame records. Placement no longer travels through it.

**Research first:**
- Each resolver's size and its dependencies on browser-only data, such as the scene-assets catalog loaded in JS.
- Whether Rust can read those catalogs directly.

Write the findings into this file before building.

**Order: web first.** Land the change in the browser first. Measure before and after with slice 07's harness on the graphics test and a medium battle: average FPS, 1% low, p95 and p99, main-thread CPU and GPU timestamps, three runs each. Only then switch native to the same code.

**Focused gates:**
- Equality: the Rust resolvers' output equals today's TypeScript output on both reel maps, record for record, before the TypeScript is deleted.
- The shipped browser draws the same frame. Plates are byte-identical, and the terrain and grass WGSL is unchanged.
- No meaningful browser regression; startup stays within the 30 s budget.
- Less code, as in 19 and 20.
- Native plates and counts unchanged.

**Delegated:** the crate placement and the record shapes.

## Research (2026-10-11)

### What each resolver is, and what it reads

Sizes are the file's own lines, then its runtime closure: every local module it imports for values, not types (`throwaway/` script, `import type` excluded).

| Resolver | Own | Closure | Resolves | Reads that only the browser holds |
| --- | --- | --- | --- | --- |
| `scene-assets/templateLibrary.ts` | 320 | 2,086 (codec, schema, material, texture, glb) | A template's rows per state, `placeRows` | Nothing: the library is one container file the catalog names |
| `models/buildingPlacements.ts` | 850 | 4,607 | Per building: bounds, tint value, coarse chunk; each row's model record. About 60% of it is the per-frame walk (residency, pool, ranges, casters) | **Module mesh bounds**, from each kit bundle's `states[].bounds`; the bundles arrive lazily as kits install |
| `models/buildingReferences.ts` | 156 | 157 | The map's buildings as columns, fallen state from `knownProps` | Nothing (the map export, and the publication) |
| `scenery/placement.ts` | 840 | 5,122 | Forest, understorey, backdrop, dressing field | **Each kind's size** (appearance bounds); the **resolved biome** (`biome.ts`, 1,130 lines of fixture resolution); the **terrain surface** (plots, surface field, rivers, strokes: about 1,900 lines) |
| `scenery/dressing.ts` | 247 | 5,814 | Almost all per frame: the cell cache, fade, cleared ground | Kind sizes |
| `terrain/forestShapes.ts` | 153 | 222 | Forest shapes from the map export | Nothing |
| `frame/terrainHeights.ts` (`packTerrainHeights`) | 15 of 72 | — | The height pages as words | Nothing (the map export) |
| `frame/terrainMaterial.ts` tables | about 430 of 2,386 (`createTerrainSource`'s world write, `packNodes`, `packPlots`, roads, forest params) | with `plots.ts` 510, `surfaceField.ts` 674, `terrainSurface.ts` 205, `terrainTuning.ts` 439, `biome.ts` 1,130 | `TerrainParams`, plots, nodes, surfaces, surface index, water, scars | The resolved biome and tuning (TS objects today); the surface field is generated in TS |
| `frame/grassPass.ts` tables, with `terrain/grassField.ts` | about 200 of 1,079, and 328 | — | Grass params, props grid, shapes, index lists | **Blade meshes**: `packGrassShapes` reads the installed grass bundles' vertices |

So the resolvers are not five files: with what they need, the static world is about **6,500 lines of TypeScript** (biome resolution, plots, surface field, terrain surface, rivers, strokes, forest shapes, scenery placement, the template rows, the packers). Each produces float32 records the plates are made of, so each port must reproduce TS float order exactly (V8's fdlibm `Math.cos`/`sin`, `mulberry32`, the `math` package's `simplex2d`). That is three slices of work, not one pass.

### Can Rust read the catalogs?

Yes:
- **The runtime catalog** (`assets/runtime/catalog.json`) is plain JSON: appearances by name with their bundle hash, the template library's hash, and `gzip`, the transport hash for each file.
- **Bundles and the template library** are one container format (`scene-assets/codec.ts`): a 16-byte header, a JSON header naming typed views, then the views. A Rust reader is about 60 lines over `serde_json`. On disk each file is stored gzipped under its transport hash, so native needs an inflater (`flate2`). The browser already inflates in JS and can hand Rust the bytes.
- **Module bounds** are in a kit bundle's JSON header (`states[].bounds`), so Rust reads them without touching mesh data. **Kind sizes** (scenery) are the same bounds. **Grass shapes** need mesh vertices, which are views in the same container.
- **The biome and tunings** are fixture JSON plus TS defaults and constants (`biome.ts`, `terrainTuning.ts`, `grassTuning.ts`). Rust can read the JSON, but the defaults and constants are TS code that would move with the port.

### Where the code goes

A new crate, `crates/static-world`: presentation's static half, beside `presentation-wasm` and the native core, depending only on `serde_json` (and `flate2` behind native's feature). Not `mapgen` or `contract`: those own the physical map and its admission, and the simulation must never see art (the template library's own rule). Not `crates/presentation`: slice 20 creates that in parallel, and the static world shares no code with per-frame posing. When 18b's `load_world` exists, it calls into this crate.

### The memory constraint that sets the record shape

*(Superseded in part: the walk moved into Rust too, so the page asks for draws, not rows; Rust still never expands a whole map, only the resident chunks.)*

Expanding every building's rows for the whole map is what native's export does: on market-town that is about 90 MB of tables (slice 15). The browser never holds that; it expands per chunk as chunks come near. So Rust resolves **per building** (frame, bounds, tint value, chunk, rows per state), and expands a set of buildings' rows in one state on request (`rows(buildings, state, tier)`): one WASM call per chunk the browser expands, and one for the coarse population at load. The records are slice 15's exported layouts unchanged (`BuildingRecord`, `BuildingModule`, 64 B each), so native's walk reads the same records with no export in between.

### Build order

1. **Buildings and their walk** (built, below): the template library reader, the per-building resolve, and the per-frame walk, in Rust for both clients; the manifest's `buildings` section and artifacts deleted.
2. **Forest shapes and the terrain surface** (plots, surface field, rivers, strokes): the inputs both scenery and the terrain tables need. Its own pass, with vectors at each stage.
3. **The biome and tunings in Rust**, then the terrain and grass tables, then scenery placement and the dressing field. The WGSL stays; only the records move.

Each step is gated alone: equality, browser before/after, native plates and counts.

## Step 1 findings: buildings and their walk (2026-10-11)

Built on `slice20b` (`91309b2a4`, `b5199a072`, `079e0fa0a`, then this record). The browser draws from Rust first, then native switched; the TS walk and the manifest's `buildings` path are deleted.

**Contract.**
- `crates/static-world` (scaffolding until it folds into `crates/presentation`): `library.rs` reads the template library and kit bundles' bounds; `buildings.rs` resolves each building (`BuildingRecord`) and places its rows in a state (`BuildingModule`); `walk.rs` is the one per-frame walk (sim space, `f64`); `files.rs` reads them from the repository for native.
- The page: `StaticWorld` (`crates/presentation-wasm/src/world.rs`) behind the renderer's `BuildingWalk` interface (`buildingLayer.ts`): `select(view words)` → changed, `records(pass)` and `draws(pass)` (library module, tier, first, count), `counts()`, `stats()`, `set_fallen()`. The layer uploads a pass's records when they change and draws ranges.
- Native: the extension's `BuildingWalk` (glue: native view to sim space, records to MultiMesh rows, casters by region); `building_layer.gd` follows what the side knows and writes the batches. The manifest no longer has a `buildings` section or artifacts; native resolves `fixtures/maps/<identity.map>/map.json`.

**Equality.**
- Resolver: Rust's tables equal the TS-exported control artifacts (`2026-10-11T18-47-00-317Z`) word for word on both reel maps: `buildings`, `intact`, `ruin`, `gutted` and the module list (market-town 3,050 buildings, 490,886 intact rows).
- Walk: counts per module and tier equal the TS walk's on every one of 400 views of each reel map (a camera tour over every tier, buildings falling), with the TS budget lifted.
- Native: on one capture (base control `2026-10-11T21-59-51-934Z`), native before (base binding) and after (this branch, the capture's `buildings` section stripped) both report no blocker, so every plate's building counts equal the browser's. Paris-corner plates: 6 of 6 byte-identical. Market-town: 15 of 18 byte-identical, the other three differ by one pixel each (1, 3 and 8 levels). A second before run of unchanged code differs from the first on plate 1482 by the same one pixel of 8 levels, so that is native's grass noise; plate 6336's one pixel of 3 levels is a level past the stated floor.
- The browser's own plates can't be compared across runs (its frame is not reproducible); its picture is held by the walk equality above and, per 18b question 2, judged in slice 23.

**Browser before/after** (production build, plugged in, interleaved; runs overlapping another session's unlocked GPU use, 14:51–15:12, discarded and replaced).

| graphics test | avg FPS (median; range) | 1% low | p95 / p99 ms | CPU ms p50 | GPU ms (timestamps) |
| --- | --- | --- | --- | --- | --- |
| market-town before | 65.6 (65.5–69.4) | 36.6 | 25.00 / 25.51 | 5.67 | 10.02 |
| market-town after | 68.8 (67.6–72.2) | 38.3 | 25.00 / 25.46 | 5.67 | 9.61 |
| paris-corner before | 102.8 (99.6–102.9) | 53.6 | 16.73 / 17.59 | 0.78 | 5.42 |
| paris-corner after | 97.1 (94.0–104.8) | 54.0 | 16.73 / 17.39 | 1.45 | 5.98 |

Metal System Trace, market-town, three 3 s traces a side: GPU busy 76.1% before and 78.0% after of wall time, so 11.60 against 11.33 ms a frame by each side's untraced frame interval; other processes' GPU share at most 9.6%.

| benchmark (city contact, 3 runs) | FPS | p95 ms | CPU p95 ms | GPU frame ms mean | prepared |
| --- | --- | --- | --- | --- | --- |
| before | 14.1, 14.8, 14.9 | 133.3–141.8 | 81.6–84.7 | 24.08–24.54 | 125–135 s |
| after | 13.9, 15.1, 14.8 | 133.2–133.4 | 71.5–80.5 | 22.31–22.68 | 125 s |

No meaningful regression. Paris-corner's main-thread CPU rises 0.7 ms at p50 (the walk and its list compare run each view change in WASM), within the frame; market-town and the battle are unchanged or better, and the battle's GPU frame is about 1.8 ms lower (likely because the walk no longer draws out-of-view records bridged between coarse ranges; not traced). Startup is within budget: the benchmark's preparation is 125 s either way. The presentation module grows from 261 KB to 406 KB.

**Net code** (comments and blanks excluded; against `35dd313c2`): TypeScript −1,049, TS tests −260, GDScript −14, GDScript tests −32, Rust +881, mjs −6, toml +13, tsx +3: **−464**. With slice 15's +270 counted against it: **−194**. Raw lines: −560.

**Left for steps 2 and 3:** forest shapes and the terrain surface; then the biome and tunings, the terrain and grass tables, and scenery placement. The README checkbox stays open.

## Step 1, folded onto slice 20 (2026-10-11)

Rebased onto slice 20 (`513bed37d`) and folded per the orchestrator's ruling (a): `82f4e0fc7`, `a42d5be54`, `6d79f8bbc`.
- **Crate.** `crates/static-world` is gone; it is `crates/presentation/src/static_world/` (`library`, `buildings`, `walk`, `files` behind the `files` feature, `batches`). The walk takes slice 20's `view::View`; `view::building_tier` sits beside `model_detail`.
- **API.** No separate WASM class. `Presentation`'s constructor arguments are optional (`new Presentation(layout?, tick_hz?)`): a static Presentation is valid with only `load_world`, and `push_publication` errors without a layout. `add_buildings(library, bounds, templates, template, frames, owners)` stages the buildings before `load_world`; `WorldInput` gains an optional `battle` (the units and effects), `cascade_far_m` and `buildings` (the style). `frame()` walks the buildings and emits them as `Frame` batches, one per kit module row, tier and pass (0 drawn, 1 caster), through a general static population (`static_world/batches.rs`) that scenery will plug into; a frame whose walk changed nothing names every batch `first = NONE`. Each kit module is one `World.appearances` row (name the kit, state the module).
- **Page: one Presentation per scene** (follow-up pass). A battle's buildings load into the battle's own Presentation (the sim client's `ObservationDecoder`); a scene with no battle gets one the viewport makes with no layout. What a scene draws is staged on it (`setBattle`, `setBuildings`) and loaded together, in one `load_world`, by its next frame. The battle stages its world as soon as it is known, and until the first publication the viewport frames the presentation for its static batches alone, so buildings draw once the battle's presentation exists (the worker is ready), behind the loading cover as before. A frame carries both kinds of batch: `Appearance.placed` marks the static world's rows, the buildings layer takes those batches and uploads only their (leading) records, the models layer the rest. A rebuilt renderer frame asks for every static batch again (`resend_statics`).
- **Fallen buildings in Rust** (follow-up pass). `frame()` reads the side's known props (`static_world/fallen.rs`, the TS `fallenBuildings` rule ported exactly) whenever the latest publication's group changes: a part replaced by a gutted-shell kind stands the building gutted; anything else, or destroyed, is a ruin, and ruin wins. `add_buildings` takes each part prop's building (the map buildings' `parts`) and the shell kinds (the catalog's `buildingRemains`). A state the template lacks fails the frame by name (`state.missing: template "…" has no "…" rows`) and changes nothing. Labs with no battle (city-block's switch, the camera lab's, the city line-up's states) hand a stand-in for the publication's known props on `ui.known`; city-ruins pushes real publications and needs none. `set_fallen`, `fallenBuildings`, `knownFallen` and `lineupFallen` are deleted; the rule's tests are Rust unit tests, the walk's fall tests go through `ui.known`, and the shelling test reads the state from a presentation fed the battle's publications.
- **Equality.** On both reel maps, 400 views each (200 before and 200 after 11 and 226 buildings fall), every view's multiset of (module, tier, pass, record) from the Frame equals the walk's: about 985,000 and 1.34 M instances.
- **Checks.** `cargo test -p presentation` (with `records_are_current`), clippy, `tsc`, the focused web tests (312), the city-block scene (its GPU-bytes baselines needed the record buffer sized by power of two and released on a new load), native `run.sh` (before the Godot freeze; native's walk glue is unchanged).
- **Performance** (one graphics-test reel under the lock): market-town 80.6 FPS, CPU p50 4.81 ms, GPU 8.30 ms; paris-corner 111.4 FPS, 1.28 ms, 4.79 ms. No regression against the step's interleaved numbers above.
- **Net code** against `513bed37d` (comments and blanks excluded): TypeScript −989, TS tests −288, TSX +27, mjs −7, GDScript −14, GDScript tests −32, Rust +1,023, toml +5: **−275**; with slice 15's +270, **−5**.
- **Godot is frozen** (user, 2026-10-11): native's plate and count gates are dropped from this pass; native keeps building, on the shared walk through its existing glue.
- **Follow-up pass checks** (rebased onto `daf0c38b4`). Equality as above on both reel maps (400 views each), the falls now driven through `ui.known` and read back from `fallen()`; `cargo test -p presentation`, clippy, `tsc`, the full web suite (1,300), city-block all PASS. The ground scene's two failing checks are not this slice's: the paused street fails with the same numbers on `daf0c38b4` without 20b (another worktree's run), and the filter check expects a rejection that `8a8365a54` (before 20b) turned into a warning and a fallback. Reel, interleaved with the parity tip (3 runs against 2): market-town 75.5 against 72.6 FPS, CPU p50 5.33 against 4.64 ms; paris-corner 100.7 against 104.5 FPS, 1.70 against 0.82 ms: the step's walk cost in Rust, as measured above, and no FPS regression beyond run noise.
- **Reduction pass** (rebased onto the parity tip `6c6e40d30`, whose light now resolves in Rust). Battle routes no longer repeat the session's fields into `LabViewport`: `useBattleSession` returns three bundles the routes spread, `viewport` (frame, presentation, appearances, GPU hook), `known` (props, buildings, camera obstacles) and `input` (pointer handlers). Building rows carry their renderer record (`Buildings::rows` yields it with its module, tier mask and building), so the walk keeps no copy and no conversion. The tip's `resolveLight` waits for the presentation module, which broke five viewport lifetime tests on the tip as well; they now wait until the frame is being built.
- **Reduction pass checks.** Equality on both reel maps, 400 views each with the falls (11 and 226 buildings through `ui.known`); `cargo test -p presentation` (59), clippy, `tsc`, the full web suite (1,298), the ground (53) and city-block (10) scenes all PASS. Reel, one run under the lock: market-town 69.9 FPS, CPU p50 5.10 ms, GPU 9.24 ms; paris-corner 107.3 FPS, 1.35 ms, 5.44 ms. CPU p50 is lower on both maps than the last run (5.33, 1.70 ms); market-town's FPS drop is on the GPU side, which this pass doesn't touch, and is within one-run noise.
- **Net code** against `6c6e40d30` (comments and blanks excluded): TypeScript −965, TS tests −287, TSX −70, mjs −7, GDScript −14, GDScript tests −32, Rust +1,169, toml +5: **−201**.

## Step 3 findings: the biome, the tables and scenery (2026-10-11)

Built on `slice20b3` over step 2 (`a3b45f27b`, `b078a3cb9`, then this record). Steps 2 and 3 land together.

**Contract.**
- `crates/presentation/src/static_world/ground` resolves the whole static ground: `biome.rs` (serde; `Biome::load(json, family)` applies the map's regional finish and refuses a bad number by its path), `tuning.rs` (the terrain and grass tunings are data, `presentation.terrain` and `presentation.grass` in `fixtures/game.json`, documented there; Rust reads the four numbers its tables use, the shaders splice the rest), `tables.rs` (height pages, plot nodes and records, `TerrainParams` but its frame `view`, the scars' look), `grass.rs` (`GrassParams` with its per-frame fields zero, the props' footprint grid), `scenery.rs` (forest, lone trees, understorey, backdrop, and the dressing laid a cell at a time).
- The page: `Ground` in `presentation-wasm`, built per map from `WorldView`'s exports, `world_layout()`, the biome file, the map's region and the fixture's `presentation` block. Calls: `terrain()` (tables), `plots(textured)`, `water_cells()`, `grass_kinds()`, `grass_params(facts, variation, tallest)`, `grass_props()`, `retune_grass(rules?)`, `scenery_kinds()`, `place_scenery(sizes)`, `trees(population)`, `forest_ids()`, `dressing()`, `dressing_cells()`, `dress(i, j)`, and `terrain_heights(...)` for the height pages fog and grass share. Probes for the ground scenes only: `plots_json`, `plot_kind_at`, `drawn_strokes`, `reach`, `field_grid`, `distances`.
- The renderer reports what only it holds: each grass kind's mesh facts (highest vertex, top, blades, root width, where its tiers start among the shape vertices, which it packs itself) and each scenery kind's size (from its installed bundle). It uploads the words as given and patches only per-frame fields (`TerrainParams.view`, the scar grid and cache words, `GrassParams`' planes, eye, window and tiles).
- The manifest no longer carries `tuning`, `terrain`, `grass` or `scenery`; its writer, validator and capture lost them, and `wordTables.ts`, `terrainTables.ts`, `grassTables.ts` and `sceneryTables.ts` are deleted. Native is untouched (Godot frozen): `godot-binding` builds and its tests pass; its GDScript reads only captures that still carry those sections.

**Equality** against the TypeScript, dumped on the base before any change (`throwaway/slice20b3/golden`): every table and placed record is bit-identical on market-town, paris-corner, river, street, geometry and ground: height pages, plot nodes, plots (textured and plain), surface records and index, water cells, `TerrainParams`, the scars' look, `GrassParams` (with the golden's per-frame fields), the props grid, forest trees and their ids, understorey, backdrop, dressing cells and every cell's pieces. With `libm`: no angle these maps use lands on one of its last-bit differences from V8. The terrain material's 29 exported functions resolve (TypeGPU) to the same WGSL on base and branch; the grass pass's spliced values are the same numbers under renamed keys.

**Browser before/after** (production build, under the GPU lock, interleaved; before is `slice20b2` at `691ea08d8`; another session's unlocked `sound` scene overlapped some runs). The uploaded tables are the same bytes, so frame cost cannot move; the runs agree.

| graphics test | avg FPS | 1% low | p95 ms | GPU ms |
| --- | --- | --- | --- | --- |
| market-town before | 66.9, 73.5 | 37.5, 38.8 | 25.00, 24.99 | 9.43, 8.97 |
| market-town after | 64.7, 78.4 | 34.4, 38.7 | 25.00, 17.76 | 9.26, 8.68 |
| paris-corner before | 96.4, 96.4 | 47.6, 54.8 | 16.79, 16.67 | 5.82, 6.07 |
| paris-corner after | 99.4, 100.2 | 58.1, 56.0 | 16.67, 16.68 | 5.76, 5.79 |

| benchmark (city contact) | FPS | p95 ms | CPU p95 ms | GPU frame ms | prepared |
| --- | --- | --- | --- | --- | --- |
| before | 14.7, 12.2 | 133.3, 158.3 | 76.9, 102.9 | 21.3, 24.0 | 124, 148 s |
| after | 13.1, 14.0 | 133.5, 133.3 | 73.2, 71.7 | 22.2, 22.4 | 121, 129 s |

Market-town's ground, tables and scenery resolve in about 0.5 s in WASM (124 ms ground and tables, 351 ms scenery; each dressing cell on request). The presentation module grows from 1.37 to 1.81 MB raw.

**Net code, steps 2 and 3 together** (comments and blanks excluded; against `82f4e0fc7`, before step 2): Rust +4,096, Rust tests +946, TypeScript −4,073, TS tests −3,343, mjs −10, tsx −20: **−2,404** (the tunings' 167 lines of fixture data aside). Step 3 alone: −3,002.

**Web suite:** green but `icons` (3 tests, the soldier models' LFS sources not pulled; identical on `origin/main` `57edae9bd`) and `viewport.test.tsx` (5 tests; identical on this branch's base `slice20b2`, passing on `origin/main`: older than step 3).

**Left:** the dressing's per-frame cache (`scenery/dressing.ts`) and the scenery layer's walk stay TS until the `World` fold moves scenery through `Frame`'s static batches.

## Step 4 findings: the `World` fold (2026-10-12)

Built on `slice20b3` over steps 2 and 3 (on `main` at `5811511d9`), then merged with `main`'s native ground and Rust scars.

**Contract.**
- The scenery walk is Rust's: `static_world::ground::walk` holds the scenery's 128 m chunks, its four tiers, the forest floor's dressing (128 cells resident, 8 laid a walk), and the felled trees' fall (`fell_pose`, the biome's felled rules). `SceneryPopulation` is a `Population` beside the buildings', so `Frame.batches` carry the scenery through the same `StaticBatches` emitter; each list carries a print, and a batch is written only when its print changed.
- `Appearance.pad` is now `draws`: 0 a model or kit module, 1 forest, 2 understorey, 3 backdrop, 4 dressing (`tint.w` its height over its fade), 5 felled (two records an instance: the tree with its fall, then squash, spread, trunk, landed, crown), 6 stump. The scenery shaders read the 64-byte `Instance`; their WGSL is unchanged, only the instance strides and offsets in `pipelines.json`.
- The ground is set on `Presentation` (`set_ground`), not a separate `Ground` class, which is deleted. Its calls carry a `ground_` prefix where the name is generic: `ground_terrain`, `ground_plots`, `ground_water_mesh`, and the probes. The grass calls keep their names: `grass_kinds`, `grass_params(shapes, density)`, `grass_window`, `grass_props`, `retune_grass`. The scenery calls are `scenery_kinds` and `place_scenery`, with four numbers a kind (height, radius, trunk radius, crown base) from the installed bundles.
- The player's settings are Presentation inputs: `set_scenery_quality(tree_lod_scale, dressing, understorey)`. So is the cleared ground, `set_cleared(runs, cols, rows, cell_m)`, which the lab can withhold. Felled trees come from the publication's fallen bodies inside Rust, so the page derives none. `scenery_pending`, `scenery_falling` and `scenery_stats` report the walk.
- `set_ground` sets the scars' view-ray reach from its own ground. The page's `set_scar_reach` is deleted. Rust keeps `set_scar_reach` for native, which holds its own `Ground`. `Presentation::ground()` (the side's learned ground) is renamed `learned()`.
- Deleted TypeScript: `scenery/felled.ts`, `lod.ts`, `dressing.ts` and `placement.ts`; the scenery layer's walk; the battle session's felled derivation.

**Equality** against the TypeScript walk, dumped on the branch before the fold (`throwaway/slice20b3/walk`): over 120 views on each of market-town, paris-corner and river, with dressing settled and a cleared pattern and felled trees set, every list matches tier by tier and kind by kind. The lists are forest, understorey, backdrop, dressing and felled, plus the casters of forest and understorey. Market-town's totals are 429,053 forest, 262,437 forest casters, 5,215 understorey, 50,564 dressing and 170,640 felled. Stumps are record for record. Felled records are identical except for 51 of 1,422, which differ by one ulp in the fall's angle during the settle (`libm`'s sin against V8's: the accepted `libm` decision). The check was rerun after the merge with `main`, with the same result.

**Browser before/after** (production build, under the GPU lock, interleaved; before is `main` at `7afb84774`, which has the TS walk). The machine's load average was 10 to 20 on 18 cores from parallel lanes, so the absolute numbers are about half of step 3's. The pairs agree within their noise:

| graphics test | avg FPS | 1% low | p95 ms | GPU ms |
| --- | --- | --- | --- | --- |
| market-town before | 19.9, 24.6 | 8.6, 7.2 | 83.90, 51.35 | 27.52, 23.49 |
| market-town after | 21.2, 29.7 | 13.0, 9.4 | 66.78, 50.05 | 19.96, 17.88 |
| paris-corner before | 35.2, 34.5 | 8.3, 8.5 | 42.13, 41.98 | 15.87, 16.49 |
| paris-corner after | 31.6, 33.2 | 7.6, 7.6 | 43.17, 41.90 | 17.59, 16.99 |

**Net code** (comments and blanks excluded; against `main`): Rust +843, Rust tests +61, TypeScript −810, TS tests −462, mjs +11, tsx +11: **−346**.

## Step 5 findings: the props (2026-10-12)

Built on `props20b` over step 4 (`003002e0c`). The map's props and what a side knows of them were the last static world built on the page.

**Contract.**
- `static_world::props` owns them. `PropArt` is the fit: a kind's candidates (its region's looks where it has any, else the shared ones, never another region's), nearest footprint by least summed log scale, a wreck its unit's own, a module repeated along the box's long side, a car's paint drawn from the authored prop it is, the stand-in box for a kind with no art. `PropPopulation` holds the map's props fitted once (bucketed in 64 m chunks), what the side knows fitted as it learns, and the walk: each model takes the units' tier rule (`view::model_detail`) by its appearance's size grown by its largest scale. It is a `Population` beside the buildings' and the scenery's, so its lists go through the same `StaticBatches` writer, view pass only; the page casts them as it casts every model, a tier coarser.
- What a side knows comes from the same place the buildings read: the latest publication's known props, or a lab's stand-in on `ui.known`, which now carries a known prop's id and box (a stand-in without a box, as the line-up's, stands buildings only). A building's part is left to its building, and a wreck the cook-off draws moving (`CookOffs::moving_props`) is left out until it lies still, read at each frame.
- `Appearance.draws` gains 7 (a prop, fogged face by face) and 8 (a surface movers stand on, which takes the ground paint). The rows are every look the map's props and a battle's leavings can need, made at `load_world`.
- The page: `Presentation.place_props(layout, art)` after `set_ground`, from the ground's exported props; `art` is `propArt` (`models/propAppearance.ts`): each installed static appearance with a footprint (scenery kind, region, footprint, paints, its first state's size and reach), each vehicle appearance's wreck, the stand-in kit's box, `presentation.stand_ins` and `presentation.models`. `prop_stats()` and `prop_models()` are the labs' probes; `prop_appearances(layout, props, art)` names what the models layer installs for them (`mapAppearances`). The decoder places them with the scenery, when its ground is the surface view (`GroundInput.props`).
- The models layer holds each prop batch in a buffer of its own until the frame writes it again and draws it as a run beside the units', in the same passes as before (units' prepass, the fog classes, the cascades).
- Deleted: `PropAppearances` and the fit, `SideStructures`, `setStructures` (scene, frame, world pass, viewport), `WorldLayers.structures`, `worldStructures`, `apartKinds` and the sessions' `destroyable` option, the models layer's statics walk, `frame/staticChunks.ts` (its last population) and the dead sun-shadow plumbing into `prepare`, `gameStandIns`. The manifest's `props` keeps only `buildings` (native reads nothing else); the map's prop table artifact (`props-map.u32`) and the fit's tables are gone, and a plate's instance counts list `props` per tier in place of `knownProps`.

**Equality** against the TypeScript, dumped on the base before any change (`throwaway/props-equality`): market-town, paris-corner and street, 120 views each (near, middle and far), with no knowledge and with a synthetic side's (every 7th destroyable or movable prop seen destroyed, every 5th shoved, every 11th rubble, a wreck of every unit type, stand-ins among them). Each view's multiset of (appearance, state, fog class, tier, record) is compared.
- The map's props: every view identical on all three maps (market-town 54,077 models, 694,846 drawn over the views; paris-corner 101; street 93).
- With knowledge: every view's counts per appearance, state, fog class and tier are equal on all three maps (market-town 587,530 drawn; paris-corner 12,389; street 6,504). The records are identical on street, and on the other two differ only where a known car wears its paint: with the TypeScript's seed routed in (a scratch build), every view of all three maps is identical record for record. The TypeScript seeded a known body's paint by its own published id, so a shoved car changed colour; Rust seeds it by the authored prop, as the rule says.
- What the models layer installs (`prop_appearances` against `drawnFor`): the same names on all three maps.
- Two fixes the scenes found, outside the props: the scenery layer held a batch's buffer for every key it had ever drawn and grew it without shrinking, so the same view held different memory by its history; it now holds only the batches a frame lists, a power of two of records each, as the props do. And the felled trees' lists named themselves unchanged when a camera move retiered a tree, so the renderer drew the last view's records; their print now follows what they hold.

**Checks.** `cargo test -p presentation` (13 new prop tests: the fit's rules, the install set, a side's knowledge, a moving wreck, the walk), clippy `-D warnings` on `presentation`, `presentation-wasm` and `native-client`, `cargo fmt`, `tsc`, oxlint (no errors) and oxfmt; the WGSL export unchanged (`shaderExport.test.ts`, no layout moved). Scenes, on the branch merged with `origin/main` at `abdb7035b`: `city-block` (10 checks), `fog-look` (8), `graphics-test` (2) and `street`'s works, vehicles and effects tours (8), all PASS. Full web suite: 1,243 of 1,256 after pulling the merge's new runtime art; the failures are not this step's: `icons` (3, the units' LFS model sources not pulled), `voiceCatalog` (8 tests: `origin/main`'s new artillery units have no voice assignment; this branch touches no voice or catalog file), `mechanicsServer` (fails on the base too) and `navigationBrowser` (its cold dev server misses the 60 s click on the base as well, two runs of two).

**Browser before/after** (production build, the graphics test's market-town, one run each under the GPU lock with the machine otherwise idle; before is `c34a9f1df`):

| market-town | avg FPS | 1% low | frame ms p95 / p99 | CPU ms p50 | GPU ms |
| --- | --- | --- | --- | --- | --- |
| before | 68.1 | 33.7 | 25.01 / 25.56 | 5.71 | 9.10 |
| after | 70.7 | 36.2 | 25.00 / 25.21 | 2.77 | 9.19 |

The main thread's frame halves: the TypeScript statics walk (54,000 map models chunked and tiered every view) is gone, and Rust walks them when the view changes. The GPU frame is the same.

**Net code** (comments and blanks excluded; against `c34a9f1df`): Rust +761, Rust tests +335, TypeScript −933, TS tests −802, TSX −54, mjs −4: **−697**.
