# Native rendering visual parity

## Status and next pickup

**Status:** phases 1 and 2 are built and closed (2026-10-10 and 2026-10-11). Native draws the menu reel from the browser's frame records with the same camera, light, terrain and grass. Its release export is no slower than the browser's production build on any measure, on both scenes (slice 12): market-town p95 13.4 ms against 23.9, and GPU busy about half the browser's on both. [`choices.md`](choices.md) holds the discovery map and both phases' ledgers.

**Next pickup:** phase 4. Slice 15's Rust selection walk (the first module of slice 18's native core) is in progress in the `slice15-perf` worktree. Slice 19's research can run beside it. Read `choices.md` first; its decisions are givens.

**Goal:** make the native Godot client visually equivalent to the browser renderer for the authored menu reel, and at least as fast as it, as a maintained second client beside the browser. Rust stays the only gameplay, observation, visibility, replay and digest authority. The browser remains shipped.

## Phases

The spec ships in three phases, and slice numbers follow build order.

- **Phase 1** built the comparison loop and ported the ground under the same camera and the same light.
- **Phase 2** makes native fast. Profiling comes first, because the first matched measurement showed native slower than the browser on an idle machine, which points at architecture rather than load.
- **Phase 3** ports the authored models and the events, then decides promotion. Its slice files are placeholders, planned with the user after phase 2.

Schema ids carry no version suffix. A format change updates its writer, reader and tests together in one change, and the strict parsers reject stale files.

### Phase 1: same camera, same light, same ground

Graph: `01 ∥ 02 → 03 → 04a → 04b → 04c → 05a → 05b → 06a → 06b → closeout`. Slices 01 (browser) and 02 (native) share no files and run in parallel. Each later slice judges one visual variable.

- [x] 01 — Browser capture run (`0b8ac2a9`): the complete frame record, the base manifest, same-frame plates, and the `native-parity-capture` scene. [slice](slices/01-browser-capture-run.md)
- [x] 02 — Native runner (`ae645e7a`): rename to `native/godot`, Forward+, the strict reader, stepping by index, the report, per-file staging, and silent fallbacks and caps deleted. [slice](slices/02-native-runner.md)
- [x] 03 — Coordinates and camera (`0d264eb5`): the first real pair; the same shot. [slice](slices/03-coordinates-and-camera.md)
- [x] 04a — Light: sun, fill and shadows (direction and softness). [slice](slices/04a-light-sun-and-shadow.md)
- [x] 04b — Light: sky, clouds and distance haze. [slice](slices/04b-light-sky-and-haze.md)
- [x] 04c — Light: output transform (tone), a formula port of the browser's post chain. [slice](slices/04c-light-output-transform.md)
- [x] 05a — Terrain: height and features (`48c3ab7a`): class agreement 100.00% with a 2 px edge mask (gate 99.5%). [slice](slices/05a-terrain-height-and-features.md)
- [x] 05b — Terrain: field character (`89481000`): furrows, wheelings, grain and mottle match the browser's grass-off `plain-<index>.png` plates at broad and close cuts. [slice](slices/05b-terrain-field-character.md)
- [x] 06a — Grass: placement and per-region counts. [slice](slices/06a-grass-placement-and-counts.md)
- [x] 06b — Grass: look and wind. [slice](slices/06b-grass-look-and-wind.md)
- [x] Closeout: [`review`](../../.agents/skills/review/SKILL.md) over phase 1's whole diff, plus an independent Codex review (`ed268e8b`, `bcaa8fff`, `ef730416`). The class-view scaffolding is removed, and the ledger is consolidated.

### Phase 2: native performance, profiling-led

The user set this phase on 2026-10-10. Visual parity is de-risked: the method of browser-exported data, ported formulas and a fresh control worked for camera, light, terrain and grass, and models are more of the same. Performance is not. A native Godot run showed 14 FPS with 76.7 ms of GPU time per frame while the machine sat nearly idle (GPU about 62%, CPU idle). That signature is waiting, not work. Native must be at least as fast as the browser, and is expected to be clearly faster.

Graph: `07 → 08 → 09 → 10 → 11 → 12`. Measure first and name the cause. 08 found it was work, not waiting: 09 rebuilds how native draws models, then 10 tunes the remaining passes and presentation. Every slice keeps the picture unchanged, within the native grass noise floor.

- [x] 07 — Profiling harness (`2fd549f2b`, `cfe6887ae`, `39850b7b7`): one command per client, one summariser, GPU traces on both; native draws every frame itself, so a hidden window no longer stalls it. Not taken: the Time Profiler CPU profile. [slice](slices/07-profiling-harness.md)
- [x] 08 — Native frame anatomy: the 77 ms frames were waiting (fixed in 07) plus the legacy layer's draws and primitives times the shadow cascades; findings in the slice. [slice](slices/08-native-frame-anatomy.md)
- [x] 09 — Native model drawing architecture (`355be5442`, then the tiers pass): `world/model_batches.gd` draws one MultiMesh per appearance and authored tier, a surface per material; statics culled and tiered by 64 m chunk, casters one tier coarser. Market-town 1000–1299: 19,922 → 410 draws, 18.0 → 4.24 M primitives, 34.75 → 8.45 ms p50. Impostor cards deferred to 13. [slice](slices/09-model-batches.md)
- [x] 10 — Per-pass budgets and presentation (`a214d9808`, `2c04e93fe`). Kit shells draw an open tier over the coarser ones and a closed tier alone, and native bakes the browser's sky lookup. Market-town reel p95 24.4 → 14.8 ms and p99 26.3 → 16.0 (browser 25.1 / 26.1); GPU busy 6.3 → 4.4 ms a frame (browser 9.8); presenting costs about 0.1 ms. New control `2026-10-11T08-56-34-958Z`. [slice](slices/10-pass-budgets.md)
- [x] 11 — Release build: `native/godot/release.sh` exports a macOS release build and runs the parity run from it; its paris-corner and market-town plates match the editor run's within the grass noise floor, and the report states its build kind. The browser's production build and the side-by-side timings are slice 12's. [slice](slices/11-release-build.md)
- [x] 12 — Performance gate: passed. Release export against the browser's production build, three interleaved runs, plugged in. Market-town averages 115.0 FPS against 71.6, with p99 14.4 ms against 25.4. Paris-corner averages 120.0 against 101.5, with p99 9.6 against 17.8. Traced GPU busy is 5.9 against 10.8 ms and 4.5 against 8.2. The plates are byte-identical to the editor's. On the way, the bench now re-minimises its window before timing: an on-screen window had put paris p50 0.03 ms over the browser's. [slice](slices/12-performance-gate.md)

### Phase 3: same models and events, then promotion

Planned 2026-10-11 by the orchestrator on the user's behalf, under the confirmed preferences in `choices.md`. Every call is reversible and recorded there.

A source map found that the capture already carries every effect cause, so no new simulation data path is needed. Since slice 13, native decodes every publication group.

The rule follows phase 1: the browser exports what is static per map, and native ports what is derived per frame.
- **Static per map, exported:** building modules with facades, and scenery placement.
- **Derived per frame, ported as formulas:** unit poses and gait, effects (seeded, on the recorded sim clock), and fallen, felled or cleared state from the publication. A live native client needs these anyway.

Every slice reports its own pass cost and keeps slice 12's bar.

Graph: `13 → {14, 15, 16} → 17 → 18`. Slices 14, 15 and 16 share no owners and run in parallel; 17 needs 13's decode and 14's posed muzzles.

- [x] 13 — Full publication decode (`785d7aaf2`): `capture/observation.gd` ports the browser's decoder, and every field of every sample of paris-corner and a seven-sample market-town window equals the browser's decode. Guided missiles, crashes, leans, encounter and skirmish appear in neither fixture (see `choices.md`). [slice](slices/13-publication-decode.md)
- [x] 14 — Units, poses, corpses and aircraft (`5f56a47ae`): interpolation, the feed, the pose driver and articulation are ported, and the browser's pose kernel runs on the GPU. Posed-model counts equal the browser's on all 24 plates, and poses match its vectors. Impostor cards are not ported (see `choices.md`). The units cost 1.9 ms at p95 on market-town. [slice](slices/14-units-and-poses.md)
- [ ] 15 — Buildings and scenery (`e41b00362`, `40425bdbc`, `ce1356947`): built. Native draws the browser's exported building modules, scenery and fitted props in the side's known state, and `legacy_map_layer.gd` is deleted. Instance counts equal the browser's on all 24 plates, and the unprimed critique found no static-world defect. **Open: the performance gate fails on market-town:** p50 22.8 ms against the browser's 15.9, and p95 35.9 against 18.2. Paris-corner passes. [slice](slices/15-buildings-and-scenery.md)
- [x] 16 — Water surface and ground scars (`98e45d17e`): native draws the side's learned scars on the ground and in the grass, and every market-town grass cell holds within 0.07% with no allowance. Water is judged by vectors against the browser's WGSL only, since no reel map has water. Scars cost about 1.4 ms at p95. [slice](slices/16-water-and-scars.md)
- [x] 17 — Effects (`8d03df43c`, `3aace2d71`, `3d2aa4eed`): native ports the browser's effect feed, launches, cook-off watch, `EffectFrame`, drawn muzzles and effect pass, with their cast lights on every world material. Effect counts per kind, shape and light equal the browser's on all 24 plates, and seeded instances match TS vectors. The effects cost about 1.3 ms at p50 and 2.9 ms at p95 on market-town. Not ported: a cook-off's moving wreck and debris (reported). [slice](slices/17-effects.md)

### Phase 4: Rust native core and one shared presentation crate

**Principle (user, 2026-10-11): the Rust side is as smart as possible, and each renderer as dumb as possible.** Rust decodes, interpolates, poses, makes effects, chooses detail and visibility, and resolves placement. TypeGPU and Godot only draw the buffers they are given. Presentation logic stays out of the simulation's authoritative state and digest. It reads only what the simulation publishes for a side, so it cannot see past fog of war, and tuning it never changes a replay.

Set by the user on 2026-10-11, after the static world's selection walk cost about 6.5 ms per frame in GDScript. GDScript is the wrong owner for the client's CPU work. Native's CPU side moves to a Rust GDExtension. Per-frame presentation logic moves into one shared Rust crate that the browser runs through WASM and native links, which removes the TypeScript-to-native duplication. Every step keeps plates and counts unchanged and the browser's shipped frame identical. The user's priority (2026-10-11) is lower overall complexity: each shared-code slice must reduce net code, and a small, measured browser performance cost is acceptable for it. A meaningful regression still blocks.

Graph: `18 → 18b (approved 2026-10-11) → 19 → 20 ∥ 20b → phase 5 (25, with 22 inside it) → 23 → 24`.

**Godot is frozen (user, 2026-10-11).** From now until phase 5 replaces it, no lane runs Godot (no plates, no display tests, no windows on the user's screen) or adds Godot glue. Native stays as it is on main. Phase 4's remaining slices stop at the browser and `crates/presentation`, and their native plate and count gates are dropped. Only headless native tests run, if a change touches code native compiles against.

**The Godot spike leaves no tech debt (user, 2026-10-11).** Phase 5 replaces Godot, so nothing Godot-shaped enters shared code. Native's halves of 19, 20 and 20b are the thinnest GDExtension glue over the shared crates, deleted with Godot. Slice 21 (moving native's remaining GDScript into Rust inside Godot) is dropped. Slice 22 (completeness) is built once, on `wgpu`, inside phase 5.

**Web first (user, 2026-10-11).** Each shared-code slice (19, 20, 20b) lands in the browser first, with clean before/after frame-rate numbers from slice 07's harness on the graphics test and a medium battle. Only then does native switch to it. Native's own core work (18, 21) proceeds independently. Slice 15's performance fix is 18's first module.

- [ ] 18 — Rust native core: run loop, readers, report, static-world selection and instance writes in the GDExtension; GDScript shrinks to an entry scene. [slice](slices/18-rust-native-core.md)
- [ ] 18b — The presentation API: a small, documented, data-oriented boundary between Rust and every renderer, reviewed by the user before 19, 20 and 20b expose anything to a renderer. [slice](slices/18b-presentation-api.md)
- [x] 19 — One publication reader in Rust beside its writer (`crates/sim/src/publication/reader.rs`), used by both clients. The page decodes through `crates/presentation-wasm` (18b's `push_publication` and `observation()`), and native through the extension's `PublicationReader`. `observation.ts` and `observation.gd` are deleted. The reader equals the TS decoder on both fixture captures and both whole control streams. The graphics test and the benchmark show no meaningful change, and the slice is net −289 lines (tables in the slice). [slice](slices/19-shared-publication-reader.md)
- [x] 20 — Shared presentation crate (`crates/presentation`): interpolation, poses (joint palettes in Rust), ghosts, launches, effects, cast lights and cook-offs, with the camera's tiers and light choice, behind 18b's `frame()`; the browser draws its records and the TS originals and GPU pose kernel are deleted. Equal to the browser's vectors; the reels faster, the battle unchanged; net −720 lines. Native adopts it in phase 5; its ports are frozen. [slice](slices/20-shared-presentation-crate.md)
- [ ] 20b — The static world resolved in Rust (building modules, scenery, terrain and grass tables, tier rules) for both clients; the TS resolvers and the manifest's placement artifacts are deleted. **Step 1 built** (`slice20b`): buildings resolved and walked in `crates/static-world` for both clients, equal to the TS tables word for word and the TS walk count for count; the TS walk and the manifest's `buildings` path deleted; no meaningful browser regression; net −464 lines (−194 with slice 15's +270). **Steps 2 and 3 built** (`slice20b2`, `slice20b3`): the ground's shapes, the biome and tunings (data in `fixtures/game.json`), the terrain and grass tables and scenery placement resolved in `crates/presentation`'s `static_world::ground`, every table and placed record bit-identical to the deleted TS on six maps; the manifest's terrain, grass and scenery sections deleted; the pair is net negative (tables in the slice). Left: the `World` fold (ground, tables and scenery behind `load_world`, scenery through `Frame`'s static batches). [slice](slices/20b-static-world-in-rust.md)
- ~~21 — The rest of native's CPU side in Rust~~: dropped (2026-10-11). Phase 5 replaces Godot, so the Godot-side GDScript is deleted with it, not ported.
- [ ] 22 — Completeness: every phase 3 "not ported" item, closed on `wgpu` inside phase 5. [slice](slices/22-completeness.md)
- [ ] 23 — The browser on shared code: whole-game check, visual suite and battle frame time. [slice](slices/23-browser-on-shared-code.md)
- [ ] 24 — Whole-frame verdict, the performance re-gate, promotion, the full repository check and closeout. [slice](slices/24-promotion-and-closeout.md)

### Phase 5: native on wgpu (planned)

Set by the user on 2026-10-11, after slice 15's Rust walk left market-town's gap on the GPU: Godot's Forward+ costs more per primitive than the browser's lean passes. Native moves to Rust `wgpu` running the browser's own WGSL over the 18b API, and Godot leaves native. It is planned in detail when phase 4 closes.

- [ ] 25 — Native draws with wgpu: the browser's shaders and passes, once for both clients; Godot's project and ported shaders are deleted. [slice](slices/25-native-on-wgpu.md)
  - Steps 1 and 2 built (branch `phase5a`): the browser's shaders are one text for every light and size, and `packages/battle-renderer/pipelines/` exports each pipeline's WGSL and settings, held to the browser's code by a Vitest drift test and validated by naga (`crates/pipelines`).
  - Step 3 built (branch `phase5b`): `native/client` builds all 63 exported pipelines on `wgpu` Metal and replays a parity scene through the presentation crate. One ordered pass list (sky, post) draws plates whose sky matches the control within 8 levels. Native's model counts equal the browser's on all 24 plates. An occluded or minimised window presents nothing on macOS. The browser now reads the same light records from Rust; its TS light derivations are deleted. Next: step 4, the ground.
  - Step 4 built (branch `phase5g`): native draws the ground through the exported pipelines: the terrain, the skirt, the backdrop and the water; the sun's four cascades, cast by the ground, the models and the buildings; the side's scars; and the grass, with its build, finish and indirect draw. Buildings come through `Frame`'s static batches. Browser first, the cascade fit (`presentation::shadow`), the ground's meshes and the grass field's CPU side (`static_world::ground`), and the learned ground with its scar cache (`presentation::scars`) moved into Rust, each equal to the deleted TypeScript bit for bit. Against a fresh control, model and building counts are equal on all 24 plates, and every remaining difference lies on the scenery and the map's props.
  - Step 5a built (branch `phase5c`): native draws every unit, corpse, ghost and moving wreck piece a `Frame` lists, from the browser's runtime bundles through the exported model pipelines (`sky → depth prepass → world → ghosts → post`). On a fresh control, counts equal `modelCounts` and every listed instance is drawn on all 24 plates; silhouettes, poses and placement match. Impostor cards and model shadow casters wait for their pipelines (the bake export, step 4's cascades).
  - Step 6 built (branch `phase5e`): native draws the effects and their cast lights through the exported `effects` pipeline, and runs fog's mask passes (`… → world → effects → fog mask → ghosts → post → fog rim`). A fog look is now a presentation record (`presentation::fog`, `FogLook`) that the browser reads too. Effect counts equal the browser's on every plate drawn between ticks; most plates drawn exactly on a tick boundary differ, and the browser's own WASM gives native's counts on the same replay (the capture doesn't pin the page's presentation inputs there). Fog of war's sight (the compute) waits for the static world's inputs.
  - Step 7 built (branch `phase5f`): what a battle hears is decided in Rust (`presentation::sound`) for both clients; the browser's TS frame is a thin carrier of its commands, and native plays them on `kira`. Rust equals every command the TS frame gave (sound lab vectors; both captures' 156,353 commands). Left: the browser's bank still synthesizes and calibrates in TS beside the Rust port; barks and battle music are not in battle on either client yet.

### Firewalls

- **No backward compatibility or migrations.** Formats change in place. A format change updates its writer, reader and tests together, and the strict parsers reject stale files.
- **Short-lived scaffolding,** each with its removal:

| Scaffolding | Removed |
| --- | --- |
| ~~`legacy_map_layer.gd` (placement, its kit shells' open tiers drawn over the coarser ones, its read of `fixtures/game.json` detail thresholds)~~ | Removed in slice 15 (`ce1356947`) |
| ~~The grass count gate's `scar-omission` allowance~~ | Removed in slice 16 |

Update this section before ending every implementation pass. Record the current slice, evidence, unresolved decisions, and the next exact pickup.

## What the finished picture must preserve

The authority is a **fresh browser control**, not the archived plates. The archived plates in `specs/done/native-rendering/visualizations/` are provenance only: the browser plate has the graphics-test HUD burned in, it was cropped from 1280×800 by an unrecorded method, and the native plate is mirrored.

Both clients render the same menu-reel workload: the same map, encounter, seed and capture, the same frame records, at 1280×720 with one pixel per CSS pixel (the browser draws at 1× on every display). Pixel identity is not required. Each slice judges one visual variable by eye, side by side:

1. **Camera and framing:** the same shot. Same subject, same side of frame, horizon in the same place to the eye. There is no pixel budget.
2. **Light, sky and fog:** the same time of day and the same weather. Sun direction, shadow direction and softness, sky gradient, cloud cover and haze read the same.
3. **Terrain:** the same features in the same places (roads, field edges, relief, forest floor), and the same field character: furrows, patchiness and wear read as the same kind of field at broad and close cuts.
4. **Grass and vegetation:** the same impression (coverage, density, colour mix, crop rows, no tiling or bald patches), and rendered blades per region within about 10% of the browser's count for the same window.
5. **Models, effects and composition:** phase 2.

A visual slice changes one primary variable. Other visible differences are explicitly out of scope until their slice; whole-frame approval belongs only to slice 24.

## Ownership and boundary contract

| Concept | Owner | Native responsibility |
| --- | --- | --- |
| Rules, fixed ticks, observations, visibility, replay, digests | Rust simulation and publication layout | Decode and draw; never simulate or infer |
| Workload identity, frame records (reel time, sim tick, interpolated sim time, which also drives wind, drawn view, canvas size), control plates | Browser graphics-test capture run | Render exactly the recorded frames; refuse a mismatched identity |
| Which presentation layers are drawn (e.g. no fog overlay in the menu reel) | Browser, recorded in the manifest | Draw only the listed layers |
| Map admission, height/relief, surfaces and authored placement | Saved-map exports and their browser resolver | Consume resolved records; never re-parse policy ad hoc |
| Appearance identity, material roles, pivots, LOD tiers | `packages/scene-assets` catalogs and the tiers the browser chose | Import the named source and preserve roles; report fallback |
| Biome, terrain, grass, light and atmosphere values | Fixture data and one exported tuning table per browser shader owner | Read the values from the manifest; port only the formulas |
| Sim-to-Godot coordinates `(x, y, z) → (x, z, −y)` | One native conversion function | Every placement, light and camera goes through it |
| Native node/material/shader translation | One Godot adapter and one frame orchestrator | Map manifest data to Godot resources and expose failures |

The seam is a `native-visual-presentation` manifest beside each `battle-presentation-capture` file, written by the same browser run. It starts in slice 01 with identity hashes, viewport, lens (`fovY`, near plane, reverse-Z infinite far), drawn layers, and fallback/omission rows. Each later slice adds only the fields native reads in that slice. Large geometry may live in hashed artifacts, but ordering and ownership stay in the manifest.

Native must not choose a building shell by family/token heuristics (removed in slice 15), re-read raw map policy, invent visibility or a presentation layer, replace a missing asset silently, cap anything silently, or add a simulation or wall clock in comparison mode. A parity report names every fallback, omission and budget with owner, reason, source id, admitted, rendered and dropped counts, and cut.

## Browser owners to port from

- Camera and projection: `packages/renderer-core/src/camera3d.ts`, `cameraUniform.ts`, `cameraController.ts`, `cameraClearance.ts`, `depthContract.ts`, `apps/battle-lab/src/gameCamera.ts`, `apps/battle-lab/src/menuReel.ts` and `apps/battle-lab/src/benchmark/reelRun.ts`.
- Capture run: `apps/battle-lab/src/routes/graphicsTest.tsx`, `apps/battle-lab/src/benchmark/{reelRun.ts,reelReport.ts}`, `web/src/battle/benchmark/presentationCapture.ts`, `web/scene.mjs` and `web/scenes/graphics-test.mjs`.
- Light and atmosphere: `crates/presentation/src/{light.rs,shadow.rs}`, `packages/battle-renderer/src/light/{sceneLight.ts,skyParameters.ts}`, `packages/battle-renderer/src/world/{environment.ts,sky.ts,shadow.ts,post.ts}` and `packages/battle-renderer/src/frame/{environmentFrame.ts,fogMaskPass.ts,fogTerm.ts}`.
- Terrain and grass: `packages/battle-renderer/src/terrain/{terrainSurface.ts,terrainGrid.ts,biome.ts,grassField.ts}` and `packages/battle-renderer/src/frame/{terrainHeights.ts,terrainMaterial.ts,grassPass.ts}`.
- Models and authored content (phase 2): `packages/scene-assets/src/{loader.ts,schema.ts,templateLibrary.ts,appearanceCatalog.ts}`, `packages/battle-renderer/src/models/*`, `apps/battle-lab/src/buildingTier.ts`.
- Effects and scars (phase 2): `packages/battle-renderer/src/effects/{effectFrame.ts,effectPass.ts}` and `packages/battle-renderer/src/frame/scarTexture.ts`.
- Native client: `native/godot/` (see its README). `parity_run.gd` is the entry point. Capture, manifest, stepping, plates and the report are separate modules, and `core/` owns the concepts they share (strict JSON, coordinates, word tables, compute passes, shader text). Models are drawn by `world/model_batches.gd`; the buildings, scenery and props by `world/static_world.gd`'s layers, from the browser's placement.

## Known evidence and risks

The closed probe proves capture decoding, authored admission, fog replay, named-cut lifecycle and display backing. It does not prove parity. Its captures and reports under `throwaway/` no longer exist; phase 1 regenerates everything from the automated run.

The browser frame is not reproducible across runs. The sim ticks in real time, and follow smoothing and the wind clock run on `performance.now()`. Parity therefore pairs a plate with the frame record from the same run, and never compares two runs.

Much of the browser's look lives in shader literals: about 320 in `terrainMaterial.ts`, plus grass and fog constants. Slices 05b and 06a lift the terrain and grass constants into exported tables before porting. Fog constants stay put, because phase 1 paints no fog. See `choices.md`.

Effects require a real publication seam (phase 2). Native must extend the browser/Rust-owned capture layout or explicitly defer an effect; it must never infer dust or smoke from camera motion or map geometry.

## Verification contract

Every visual slice produces a browser/native pair from the same frame records, judged on its one variable. Run [`compare-screenshots`](../../.agents/skills/compare-screenshots/SKILL.md) against the fresh control, then [`screenshot-critique`](../../.agents/skills/screenshot-critique/SKILL.md) with an unprimed reviewer as the last visual check. Open shots with [`preview-shots`](../../.agents/skills/preview-shots/SKILL.md) at the non-blocking human checkpoint. The agent accepts the slice; if no response arrives, record the evidence-based decision and continue.

Godot 4.7.2 records no GPU timestamps on Metal, so native GPU time comes from Metal System Trace (slice 07). Performance is gated in phase 2 (slice 12). Only macOS with Metal is verified.

### Unattended capture loop

Each side has one command, and neither needs a human.

- **Browser control.** A headless-browser scene runs `/graphics-test` at 1280×720, passing the viewport explicitly because the harness defaults to 800 high. The page must stay visible, and the run takes the GPU lock and waits on scene readiness, not on timeouts. On chosen capture frames (shot midpoints, outside fades), the page reads back the canvas texture right after submitting that frame. That excludes the DOM HUD and the fade veil. It hands the PNG and that frame's record to the harness. One run writes, per scene, the capture, the manifest and the plates into scratch.
- **Native plates.** Godot's dummy renderer under `--headless` draws nothing and never signals `frame_post_draw`, so it cannot produce screenshots. The agent launches the normal display server with the window off-screen (`--window-position 10000,10000`). Run that way, Forward+ on Metal renders and reads back an exact 1280×720 PNG in about 6 s. Native steps the frame records by index, setting camera and sim time from each record, and writes the plate for each capture frame. `--write-movie <dir>/f.png --fixed-fps 30` is available for fixed-step sequences. It needs a logged-in macOS session and takes the same GPU lock as the browser scenes. This is display-backed evidence, not headless evidence.

Native grass plates differ by one or two pixels of at most 2 levels between reruns of unchanged code, so byte comparisons of native plates need that floor.

Counters never prove pixels. Headless Godot proves lifecycle only. Every pass keeps the Rust digest/replay tests, the capture parser tests, the native lifecycle tests and the browser scenes it touches green. The full repository check and verification suite run once, at slice 24.

## Next Agent Prompt

*Updated 2026-10-11.* Phases 1 and 2 are closed. The next step is **planning phase 3** (models, effects, promotion) with the user. Don't build any of it before the user has agreed the plan.

1. **Read first:** this README, then `choices.md` (its decisions are givens), then the placeholder slices 13–16 and slice 12's findings (the budgets phase 3 inherits). Plan with [`write-spec`](../../.agents/skills/write-spec/SKILL.md).
2. **The baseline to hold.** Evidence is in `throwaway/perf/gate/` (`summary.md`).
   - Release export against the browser's production build:
     - market-town: p95 13.4 ms against 23.9, GPU busy 5.9 ms a frame against 10.8;
     - paris-corner: p95 9.2 ms against 16.8, GPU busy 4.5 ms against 8.2.
   - Native's frames are display-paced at 8.3 ms. Its headroom is GPU busy time, read from proportions-calibrated Metal System Traces (`throwaway/perf/gate/gpu.mjs`).
   - Each phase 3 layer reports its own pass cost, and slice 12's gate re-runs when the phase closes.
3. **Measuring conditions,** whenever a slice measures:
   - Every comparison is plugged in, at 1280×720, three runs interleaved, with the spread shown.
   - The browser can't be uncapped (slice 07). Native runs uncapped, and its bench re-minimises its window before timing. Check that every report's `window` reads `minimized`: an on-screen window moves a median at the display step.
   - Nothing appears on the user's screen: Godot runs minimised and Chromium headless.
   - xctrace lives at `/Applications/Xcode.app/Contents/Developer/usr/bin/xctrace`.
   - The control is `throwaway/native-parity/2026-10-11T08-56-34-958Z`, with native's staging imported fresh from the same tree. A new control and a fresh import go together.
4. **The picture must not change** except in the slice's one variable. Plates stay byte-identical or within the native grass noise floor (1–2 px, at most 2 levels).
5. **After each slice,** update this section and add the slice's choices to `choices.md`.

**Commands:**
- Control run: `PARITY_SCENES=paris-corner lockf -k <main>/throwaway/gpu.lock bun run --cwd web scene -- native-parity-capture`. One scene takes about 55 s; both take about 3 minutes.
- Native run: `godot --path native/godot --window-position 10000,10000 -- --run <dir> --scene <map>`, under the same lock.
- Native tests: `native/godot/tests/run.sh`, under the lock.
- Native release run: `native/godot/release.sh --run <dir> --scene <map> [...]`, under the same lock; it exports, then runs the same arguments from the exported app.

**Worktree prerequisites:**
- Symlink `web/node_modules` to the main checkout's.
- Set `CARGO_TARGET_DIR=throwaway/target`.
- Build wasm and the mechanics validator.
- `git lfs pull --include="assets/runtime/**" --include="assets/third-party/effects/**"`.

**Hard rules, unchanged:**
- Never compare against the archived plates.
- Never let native fall back, cap or skip anything silently.
- Keep the browser shipped and Rust authoritative.
