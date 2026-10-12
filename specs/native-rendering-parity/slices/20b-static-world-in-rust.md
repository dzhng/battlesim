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
- **Follow-up pass checks** (rebased onto `daf0c38b4`). Equality as above on both reel maps (400 views each), the falls now driven through `ui.known` and read back from `fallen()`; `cargo test -p presentation`, clippy, `tsc`, the full web suite (1,300), city-block all PASS. The ground scene's two failing checks (the filter rejection, the paused street) fail identically on `daf0c38b4` without 20b, so they are not this slice's. Reel, interleaved with the parity tip (3 runs against 2): market-town 75.5 against 72.6 FPS, CPU p50 5.33 against 4.64 ms; paris-corner 100.7 against 104.5 FPS, 1.70 against 0.82 ms: the step's walk cost in Rust, as measured above, and no FPS regression beyond run noise.
