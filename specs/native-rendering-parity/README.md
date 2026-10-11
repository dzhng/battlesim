# Native rendering visual parity

## Status and next pickup

**Status:** phase 1 is built and closed (2026-10-10). Native draws the menu reel from the browser's frame records with the same camera, light, terrain and grass, and every slice passed its one-variable gate. [`choices.md`](choices.md) holds the discovery map and the consolidated phase 1 ledger. Phase 2, native performance, is planned (slices 07–12) and comes before any more visual work.

**Next pickup:** slice 07 (profiling harness), then 08 (native frame anatomy). Read `choices.md` first; its decisions are givens.

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

Graph: `07 → 08 → 09 → 10 → 11 → 12`. Measure first, name the cause, then remove waits before tuning passes. Every slice keeps the picture unchanged, within the native grass noise floor.

- [ ] 07 — Profiling harness: one command per client, one summariser, Metal System Trace and Time Profiler on both. [slice](slices/07-profiling-harness.md)
- [ ] 08 — Native frame anatomy: pass-by-pass timeline and ablation beside the browser's, naming the cause of the slow frames. [slice](slices/08-native-frame-anatomy.md)
- [ ] 09 — Pipelined frames: no CPU waits on the GPU and no idle GPU between passes. [slice](slices/09-pipelined-frames.md)
- [ ] 10 — Per-pass GPU budgets: every native pass at or below the browser's. [slice](slices/10-pass-budgets.md)
- [x] 11 — Release build: `native/godot/release.sh` exports a macOS release build and runs the parity run from it; its paris-corner and market-town plates match the editor run's within the grass noise floor, and the report states its build kind. The browser's production build and the side-by-side timings are slice 12's. [slice](slices/11-release-build.md)
- [ ] 12 — Performance gate: native no slower than the browser on any measure, on both scenes, plugged in. [slice](slices/12-performance-gate.md)

### Phase 3: same models and events, then promotion (placeholders)

- [ ] 13 — Authored model identity, material roles, pivots, fit, recorded LOD tiers, buildings and props. Replaces `legacy_map_layer.gd`.
- [ ] 14 — Observed dust, smoke, muzzle/tracer, blast, scar and grounding effects from published causes. This needs a new publication seam: the capture must carry effect causes.
- [ ] 15 — Compose all layers within phase 2's budgets.
- [ ] 16 — Promotion gate; the full repository check runs once here.

### Firewalls

- **No backward compatibility or migrations.** Formats change in place. A format change updates its writer, reader and tests together, and the strict parsers reject stale files.
- **Short-lived scaffolding,** each with its removal:

| Scaffolding | Removed |
| --- | --- |
| `legacy_map_layer.gd` | Slice 13 |
| The grass count gate's `scar-omission` allowance | Slice 14, when native draws learned scars |

Update this section before ending every implementation pass. Record the current slice, evidence, unresolved decisions, and the next exact pickup.

## What the finished picture must preserve

The authority is a **fresh browser control**, not the archived plates. The archived plates in `specs/done/native-rendering/visualizations/` are provenance only: the browser plate has the graphics-test HUD burned in, it was cropped from 1280×800 by an unrecorded method, and the native plate is mirrored.

Both clients render the same menu-reel workload: the same map, encounter, seed and capture, the same frame records, at 1280×720 with one pixel per CSS pixel (the browser draws at 1× on every display). Pixel identity is not required. Each slice judges one visual variable by eye, side by side:

1. **Camera and framing:** the same shot. Same subject, same side of frame, horizon in the same place to the eye. There is no pixel budget.
2. **Light, sky and fog:** the same time of day and the same weather. Sun direction, shadow direction and softness, sky gradient, cloud cover and haze read the same.
3. **Terrain:** the same features in the same places (roads, field edges, relief, forest floor), and the same field character: furrows, patchiness and wear read as the same kind of field at broad and close cuts.
4. **Grass and vegetation:** the same impression (coverage, density, colour mix, crop rows, no tiling or bald patches), and rendered blades per region within about 10% of the browser's count for the same window.
5. **Models, effects and composition:** phase 2.

A visual slice changes one primary variable. Other visible differences are explicitly out of scope until their slice; whole-frame approval belongs only to slice 16.

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

Native must not choose a building shell by family/token heuristics (removed in slice 13), re-read raw map policy, invent visibility or a presentation layer, replace a missing asset silently, cap anything silently, or add a simulation or wall clock in comparison mode. A parity report names every fallback, omission and budget with owner, reason, source id, admitted, rendered and dropped counts, and cut.

## Browser owners to port from

- Camera and projection: `packages/renderer-core/src/camera3d.ts`, `cameraUniform.ts`, `cameraController.ts`, `cameraClearance.ts`, `depthContract.ts`, `apps/battle-lab/src/gameCamera.ts`, `apps/battle-lab/src/menuReel.ts` and `apps/battle-lab/src/benchmark/reelRun.ts`.
- Capture run: `apps/battle-lab/src/routes/graphicsTest.tsx`, `apps/battle-lab/src/benchmark/{reelRun.ts,reelReport.ts}`, `web/src/battle/benchmark/presentationCapture.ts`, `web/scene.mjs` and `web/scenes/graphics-test.mjs`.
- Light and atmosphere: `packages/battle-renderer/src/light/{sceneLight.ts,physicalEnvironment.ts,aerialParameters.ts,skyParameters.ts,cascadePolicy.ts}`, `packages/battle-renderer/src/world/{environment.ts,sky.ts,shadow.ts,post.ts}` and `packages/battle-renderer/src/frame/{environmentFrame.ts,fogMaskPass.ts,fogTerm.ts}`.
- Terrain and grass: `packages/battle-renderer/src/terrain/{terrainSurface.ts,terrainGrid.ts,biome.ts,grassField.ts}` and `packages/battle-renderer/src/frame/{terrainHeights.ts,terrainMaterial.ts,grassPass.ts}`.
- Models and authored content (phase 2): `packages/scene-assets/src/{loader.ts,schema.ts,templateLibrary.ts,appearanceCatalog.ts}`, `packages/battle-renderer/src/models/*`, `apps/battle-lab/src/buildingTier.ts`.
- Effects and scars (phase 2): `packages/battle-renderer/src/effects/{effectFrame.ts,effectPass.ts}` and `packages/battle-renderer/src/frame/scarTexture.ts`.
- Native client: `native/godot/` (see its README). `parity_run.gd` is the entry point. Capture, manifest, stepping, plates and the report are separate modules, and `core/` owns the concepts they share (strict JSON, coordinates, word tables, compute passes, shader text). The pre-parity buildings, props and units live in `world/legacy_map_layer.gd` until slice 13.

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

Counters never prove pixels. Headless Godot proves lifecycle only. Every pass keeps the Rust digest/replay tests, the capture parser tests, the native lifecycle tests and the browser scenes it touches green. The full repository check and verification suite run once, at slice 16.

## Next Agent Prompt

*Updated 2026-10-10.* Phase 1 is closed. You are implementing **phase 2, native performance**, starting at slice 07.

1. **Read first:** this README, then `choices.md` (its decisions are givens), then the slice file. Load `audit-performance`, `renderer` and `write-tests`.
2. **Measure before changing anything.** 07 builds the harness and 08 names the cause.
   - An FPS-comparison agent has already started 07: a native `--bench` timing mode, raw per-frame intervals summarised by the browser's code, and Metal HUD logging. It was redirected to 08's ablation once the slow frames showed up. Check `throwaway/perf/` and `git log` for what landed.
3. **Measuring conditions:**
   - Every comparison is plugged in, at 1280×720.
   - Both clients are uncapped, or both capped at the display rate.
   - Each figure is three runs, with the spread shown.
   - Nothing appears on the user's screen: Godot runs off-screen and Chromium headless.
   - xctrace lives at `/Applications/Xcode.app/Contents/Developer/usr/bin/xctrace`.
4. **The picture must not change.** Plates stay byte-identical or within the native grass noise floor (1–2 px, at most 2 levels).
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
