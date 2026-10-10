# Native rendering parity — phase 1 map

The discovery walk for phase 1 produced this map. Slice numbers here follow the final cut in the README: 01 browser capture, 02 native runner, 03 camera, 04a–c light, 05a–b terrain, 06a–b grass, 07 models. Each decision is a given for the builder. It records who closed the decision: the user, the territory (the code answered it), or the agent on the user's behalf under the delegation below. Append decisions made during the build under "Build log" with the same attribution.

## Known knowns: the settled ground

**Capture and parser today**
- `battle-presentation-capture` holds workload identity, `tickHz`, `layout`, `samples[]` and `frames[]` (`web/src/battle/benchmark/presentationCapture.ts:6-37`).
- Each frame records only `camera {target:[x,y], distance, yaw, pitch}`. It drops target height, FOV, near plane, viewport and DPR.
- The native parser checks exactly that shape (`native/godot-spike/presentation_capture.gd:127-140`).

**Browser camera**
- Z-up and right-handed (`packages/renderer-core/src/camera3d.ts:6`).
- Vertical FOV of 0.8 rad (`fixtures/game.json:780`), near 1 m, reverse-Z with an infinite far plane (`camera3d.ts:120-140`, `depthContract.ts`).
- The target follows the ground (`cameraController.ts:132-141`). A clearance spring moves the eye out of buildings and terrain (`cameraClearance.ts`). Follow shots lag 0.5 s on wall-clock time (`reelRun.ts:45-86`).

**Native camera**
- Godot's default 75° FOV.
- A step function over captured frames, with no target height, clearance or lag (`reel.gd:225,1261-1286`).

**The mirror**
- Native maps sim `(x,y,z)` to Godot `(x,z,y)`, which is a reflection (`reel.gd:950,1168,1286`). In the archived plates, buildings and skyline swap sides.

**Fog overlay**
- The browser menu reel and graphics test draw no fog overlay: neither `MenuBackdrop.tsx` nor `graphicsTest.tsx` passes `fog` to `LabViewport`.
- Native draws hidden fog cells as dark translucent boxes, capped at 4096 (`reel.gd:858-894`). That is the "water" in the archived native plate. `fixtures/maps/market-town/map.json` has no water.

**Display density**
- Since `ace81f81` the browser battle canvas draws one pixel per CSS pixel on every display. Building LOD tiers size to canvas pixels (`buildingTier.ts`).

**The archived evidence**
- The browser control plate has the graphics-test HUD burned in, and it was cropped and downscaled by hand from 1280×800 @2×.
- The captures and reports under `throwaway/` are gone.

**Running the browser control**
- The full graphics-test run was manual: run, then download (`graphicsTest.tsx:215-224`).
- `web/scene.mjs` can open any viewport but defaults to 1280×800.

**Godot**
- Godot 4.7.2 is on PATH.
- An off-screen window run renders Forward+ on Metal and reads back an exact 1280×720 PNG in about 6 s. Movie Maker mode writes fixed-step frames. `--headless` cannot produce images.

## Known unknowns: the decision ledger

### Preferences and delegation

The user confirmed these preferences and delegated matching questions for this walk:

1. Add no machinery the user didn't ask for.
2. Don't defer fidelity just to keep scope small.
3. Single owner: the browser resolves and records, native draws.
4. Automate the evidence loop.
5. Order phases by visible payoff.

Taste and intent questions stayed with the user.

| # | Question | Decision | Why | Closed by |
| --- | --- | --- | --- | --- |
| Q0 | Scope | Phase 1 covers backend and capture, camera, lighting, terrain and grass (slices 01–06b). Phase 2 covers models, effects, composition and promotion (07–10). Slice numbers follow build order. | Same camera, light and ground first | User |
| Q1 | Who produces the control and manifest | One automated headless-browser run of `/graphics-test` at 1280×720 writes the capture, manifest and plates per scene | Reproducible; no manual clicking | User |
| Q2 | What a frame records | The resolved drawn view (`eye[3]`, `target[3]`, `up[3]` after clamping, clearance and lag), plus reel time, sim tick, interpolated sim time and canvas size. Wind reads the same interpolated sim time, so it has no separate field. The lens is recorded once per scene in the manifest. | Native ports no rig logic | User (base); agent added the clock and size fields from L2 under #3 |
| Q3 | Schema versions | None. Ids are `battle-presentation-capture`, `native-visual-presentation` and `godot-render-report`, changed in place. | The user doesn't read these files; only two ids had suffixes | User |
| Q4 | How the manifest grows | Per slice: each slice adds only what native reads | No unread fields | User |
| Q5 | Lighting during phase 1 | Lighting (04a–c) comes before terrain and grass | Judge colour, don't defer it | User |
| Q6 | Viewport and density | 1280×720 at 1× on both sides, always. This is what players see since `ace81f81`. | Simplest | User; confirmed by the territory |
| Q7 | Backend | Forward+ (Metal). Delete `gl_compatibility` and keep no fallback path. The slice 02 probe (one building, a grass multimesh, a shadowed sun, the sky shader, captured unattended) must pass; failure is a named blocker. | #2: shadows, AO and fog need it. #1: one backend. The probe already rendered. | Agent on the user's behalf (#1, #2) |
| Q8 | Phase 1 closeout | Run `review`, update the handoff, then plan phase 2 with a fresh walk. The full repo check runs once, at slice 10. | AGENTS.md: everything runs once at the end of the spec | Agent on the user's behalf (#1); territory |
| Q9 | The mirror fix | One conversion function `(x,y,z) → (x,z,−y)`. Every placement, the sun and the camera go through it; one test pins asymmetric known points. Slice 03. | It's a reflection today | Territory |
| Q10 | Same battle | The manifest carries the map, encounter, seed and capture hashes. Native refuses to render on any mismatch. | The user asked that the menu video use exactly the same map | Agent on the user's behalf (#3) |
| Q11 | Same layers | The manifest lists the presentation layers the browser drew. For the menu reel the fog overlay is off: native decodes fog but doesn't paint it. | The native fog boxes read as water | Agent on the user's behalf (#3) |
| L1 | Shader-only constants | Lift every tuning constant out of the terrain, grass and fog shader modules into one exported TS table per owner. The browser shader and the manifest both read it. Native hand-ports only the formulas. Grass sampling is ported bit-exact, and a test compares hashed sample positions for fixed tile coordinates against values exported from TS. | Drift is limited to formula changes; grass drift is tested | User |

Nothing is OPEN.

## Unknown knowns: what the user's taste set

| Question | Answer | What it changed |
| --- | --- | --- |
| Camera match | The same shot by eye; no pixel budget | Slice 03 has no anchor tolerances. One conversion unit test remains as a regression test. |
| Light match | The same time of day and weather, by eye | Slices 04a–c have no colour-difference masks |
| Terrain match | The same features and the same field character | Slice 05b judges furrows, patches and wear at broad and close cuts |
| Grass match | The same impression, plus blade counts within ~10% per region | The manifest exports per-region counts in slice 06a; native reports its own |
| Endgame | A second client, both maintained | Native code is long-lived: rename to `native/godot`, split `reel.gd` by concern, write behaviour tests |
| Platform | macOS only for now | Only Forward+ on Metal is verified |
| Who accepts | The agent, non-blocking | As the verification contract says |
| Performance | Record GPU frame time, don't gate | Every native report from slice 02 on. Godot 4.7.2 has no Metal GPU timestamps, so the field is null (build log). |

## Unknown unknowns: landmines

Each landmine is either decided (closed by the agent on the user's behalf unless marked otherwise) or a sharp edge.

| # | Landmine | Evidence | Status and decision |
| --- | --- | --- | --- |
| L1 | The browser's look lives in shader literals | `terrainMaterial.ts:267-326`, `grassPass.ts:228-237,583-597`, `fogTerm.ts` | Decided by the user; see the ledger |
| L2 | The browser control isn't reproducible across runs | `reelRun.ts:45-55,73,132`, `useBattleSession.ts:685`, `grassPass.ts:584` | Decided (#3, #4). A plate and its frame record come from the same run and the same frame. The page reads back the canvas texture right after submitting the chosen frame, which excludes the HUD and fade veil (DOM). Native renders the record. The run waits on `scene.settled()`, never on timers. |
| L3 | A new camera shape is rejected, and the reel silently falls back to the authored orbit | `presentation_capture.gd:127-140`, `reel.gd:1248-1259` | Decided (#1). In comparison mode an invalid capture is a named blocker; delete the fallback. |
| L4 | Silent caps | `reel.gd:31` (256 buildings), `:66,392` (2048 props per kind across maps), `:836` (512 grass), `:544` (radius around targets), `:381` (`break` on a bad row) | Decided (#2). No default caps. Budgets report admitted, rendered and dropped counts. Cull by frustum and eye distance. Skip and report bad rows. |
| L5 | The mirror flip touches about 17 sites | Road sign `-atan2` (`:333`); ring meshes with no normals and mixed winding (`:708-759`); shader `xz` (`:615,677`); sun in Godot space (`:200`) | Decided (#3). One conversion function, normalised winding, up normals, converted world coordinates for shaders. Slice 03. |
| L6 | Hard-coded 30 Hz; a hitch skips cuts | `reel.gd:1121,1093` | Decided (#3). Native steps frame records by index; no wall clock in comparison mode. |
| L7 | The import pipeline turns GLBs into primitives silently; caches aren't ignored; `user://` is shared across worktrees | `reel.gd:503,523-535,1430` | Decided (#4, AGENTS). An ignored staging folder of symlinks plus one import step. A missing import is a named blocker. Explicit report paths in scratch. A project `.gitignore`. |
| L8 | Tests pin implementation details and inherit env | `reel_lifecycle_test.gd:17,30-35,61-99` | Decided (AGENTS). Rewrite against behaviour as slices land, and clear `GODOT_*` in tests. |
| L9 | "Spike" naming and a monolith for a maintained client | `native/godot-spike/reel.gd` (1,460 lines) | Decided (#1, endgame). Rename in slice 02; split by concern as slices touch it. |
| L10 | Grass reach is in metres per pixel | `grassField.ts:288-302`, `grassPass.ts:872,922` | Sharp edge, slice 06a. The canvas height must match; the frame record carries it. |
| L11 | Flat-ground assumptions | Centimetre layer offsets at `reel.gd:291-404`; `max(0,z)` at `:1168` | Sharp edge, slice 05a. Seat everything on one height function; no stacking by offsets. |
| L12 | Harness defaults | `web/scene.mjs:185-186`, `web/scenes/graphics-test.mjs:5` (800 high); a hidden tab fails (`graphicsTest.tsx:167`); preparation can saturate the GPU for minutes | Sharp edge, slice 01. Pass 720, keep the page visible, take the GPU lock, wait on readiness. |
| L13 | Building tiers depend on canvas pixels | `buildingTier.ts` (`ace81f81`) | Sharp edge, slice 07. The manifest records the chosen tier. |
| L14 | Slice 04a shadow cuts use unported buildings | Token-picked shells until slice 07 | Sharp edge, slice 04a. Judge shadow direction and softness, not shapes. |
| L15 | Small leaks | A material per unit per frame (`reel.gd:1159`); units reused by index rather than id (`:1165`) | Sharp edge, slice 02. Fix in passing. |

## Confirm before coding

- The slice 02 Forward+ probe has to pass on the real market-town content, not just the cube probe in `throwaway/godot-shot-probe`.
- WebGPU canvas readback needs `COPY_SRC` usage on the canvas configuration. Check that `LabViewport` can add it without changing the shipped frame.
- Check whether Godot imports symlinked GLBs under `res://`. If it doesn't, stage copies into the ignored folder instead.

## Plan synthesis (`write-spec`)

Four independent drafts produced the cut, each with a different bias: fewest slices (Claude), risk first (Codex), seam quality (Claude) and feedback speed (Claude). The decisions below were taken from them. All were closed by the agent on the user's behalf.

| Decision | Why | Alternative not taken |
| --- | --- | --- |
| Slices 01 (browser) and 02 (native) run in parallel and join at 03 | They share no files. A native runner can be built against a hand-written capture. | One merged "same shot" slice (fewest-slices draft): fewer rungs, but it serialises two independent lanes |
| The frame record is complete in 01 | Later slices change only the manifest, so nothing changes the capture format twice | Grow the record per slice |
| No separate wind-clock field | Wind reads the interpolated sim time (`useBattleSession.ts:714`, `grassPass.ts:584`), so a second field would duplicate one owner | Two equal fields with a test that they're equal |
| `samples[].camera` is dropped | Nothing reads it | Keep it |
| Plates come from the existing `/graphics-test` route with `?plates=1` and `?scenes=`, driven by a new `native-parity-capture` scene | Reuses the reel run. Readback cost lands only in plate mode, which marks frame timing as not comparable. | A separate route that duplicates the reel page |
| Light splits into 04a sun/shadow, 04b sky/haze and 04c tone | The taste row "same time of day and weather" hides three variables. Tone comes last, because it can only be judged once scene radiance matches. (The first cut put tone first; corrected while reading `post.ts`.) | One light slice (fewest-slices draft) |
| Terrain splits into 05a features (with a measured class-agreement gate) and 05b field character | Two variables. The browser already has a class view, so features can be measured. | Height as its own slice. Neither reel map has relief, so a TS-vector test suffices. |
| Terrain ships the browser's packed buffers as hashed artifacts, uploaded through `R32_UINT` `Texture2DRD` | Avoids porting about 2,300 lines of plot, biome and surface logic. Godot spatial shaders have no storage buffers, and `Image` has no u32 format. | Port the resolvers |
| Fog constants are not lifted in phase 1 | Phase 1 paints no fog overlay, so nothing would read the table. This amends L1's scope. | Lift them now |
| The grass build path is decided by a probe at the start of 06a: compute writing the MultiMesh, or the browser's vertex-stage cull | Compute-written MultiMesh rendering had bugs in Godot 4.4 (godotengine/godot#105100) | Pick one now |
| The far plane is finite: the map diagonal plus `backdrop.reach_m`, reported as a budget | Godot `Camera3D` has no infinite far plane or custom projection. Reverse-Z has been built in since 4.3. | — |
| Tests call `quit(1)` rather than `assert`; staging links one file at a time | Verified on 4.7.2: a failing `assert` hangs script mode, and directory symlinks write `.import` files into `assets/` | — |
| The capture run fails fast on a resource abort | Measured: it hung in `running` on an LFS-pointer bundle | — |
| Phase 2 renumbers to 07–10 | Phase 1 now has sub-slices | — |

## Build log

Append decisions made during the build here, with their attribution.

### Slice 01 (`0b8ac2a9`), by the implementing agent; reviewed by the orchestrator

| Decision | Why |
| --- | --- |
| The scene hashes the written `capture.json` and `map.json` bytes from disk; the page supplies only lens, layers and omissions | Identity is the bytes native will read |
| Plates are held in the page as PNG blobs and pulled by the scene at the end | Avoids coupling the route to a harness binding |
| A plate frame is the first frame at or past each shot's middle | `validateReel` already fits both fades inside a shot, so the middle is never veiled |
| The layer vocabulary is a fixed list of 13 names, of which the reel draws 11 (no `fog`, no `overlays`) | Native filters on the same names |
| `?scenes=` refuses an unknown name; a single-scene run films the same camera rig as a full run | No silent subset; the same framing either way |
| The route listens to `appResources` to report a resource abort | The app's error boundary unmounts the route, which used to leave it hanging in `running` |
| The worktree pulls `assets/runtime/**` and `assets/third-party/effects/**` (about 417 MB) | The reel needs 321 of 337 bundles |
| The lens records `far: "infinite"` without checking it against the drawn camera | Today's camera never sets a far plane |

### Slice 03 (`0d264eb5`), by the implementing agent; reviewed by the orchestrator

Framing passes: 18 of 18 market-town pairs and 5 of 6 paris-corner pairs (the sixth is all explosion). Edge maps align within 4 px apart from grass-dominated frames, and the unprimed critique agreed.

| Decision | Why |
| --- | --- |
| The far plane is the map diagonal plus the manifest's `light.backdrop.reach_m`; the sun comes from `light.sunDirection` | The manifest is the seam; no second fixture read |
| Legacy terrain and sky shaders read sim (x, y) world coordinates | One conversion owner |
| Rotated boxes scale in their own axes | `scaled()` skewed them |
| Units draw at their authored size (the legacy 1.8× scale is gone) | It doubled every vehicle and misframed subjects |
| A `Logger` turns every logged script error into a blocker, and `tests/run.sh` fails any test that logs one | GDScript runtime errors return null and carry on silently. This exposed two load-skipped-model bugs. |
| The test fixture is paris-corner trimmed through its first plate (580 KB, artifacts included), with a frozen map copy | A real capture replaces the hand-written one |
| Infantry is mostly missing because only the own-side group is decoded, one model per unit | Phase 2 (07) owns squads, enemies and corpses |

Defects found for later slices: tree lines and forests drawn as flat bands (vegetation is phase 2 unless 06 adopts it); wrecks, infantry and a T-posed soldier (07); balcony railings and lamp-post height (07); road markings and decals (05a/05b surfaces, or 08 for scars); effects (08); oversized grass (06).

### Browser lanes built ahead of their native slices (`8209a90b`, `449e9d8e`, `8b101db5`, `5d6898ad`)

The orchestrator built the browser halves of 04, 05 and 06a early, in parallel with slice 03. This bends Q4 ("each slice adds only what native reads") in timing only: every field has a phase 1 reader.

| Decision | Why |
| --- | --- |
| The manifest's `light` is the light the battle frame was built with, plus a page-derived unit `sunDirection`. The Node writer checks shape only. | The scene harness runs the writer under plain Node, which can't import the renderer (`449e9d8e` fixed the break `8209a90b` introduced) |
| Tuning numbers (scales, frequencies, weights, thresholds, colours) are lifted; hash multipliers, decorrelation offsets, lattice rotations, Rec. 709 weights and the rut profile's derived constants stay inline | The latter are part of the formula, which native ports as text |
| `WATER_ROUGHNESS` and `WATER_SHADOW` remain as aliases of `terrainTuning.water` | `worldPass.ts` imports them. Collapse them at the phase 1 review. |
| Terrain artifacts are the exact buffers the material uploads, with layouts generated from the GPU struct definitions | One packer; the layout can't drift |
| Grass counts are clumps whose root projects on screen (occlusion ignored), in 4×3 cells, read in the plate's own frame. `grassPass.probes.clumps()` now queues both copies at the call. | A later frame could slip in between the old copy and its wait |
| Readbacks run only in plate mode; terrain tables are taken once, at the first plate | Debug readbacks cost frame time |
| Both reel maps are flat (the height grid has no pages); non-flat terrain is covered only by tests | It's the territory |
| `tuning: {terrain, grass}` is passed by the page; the validator checks only that the values are finite numbers or tuples | The Node writer can't import the renderer |
| Grass tables come from one frame accessor, `uploadedTables() → {terrain, grass}`, read at the first plate. `params` keeps the view fields from the last regrow, and native overwrites them per frame. The index strips are exported as uploaded (padded u16). | One packer; exactly what the GPU read |
| Generic word-table code lives in `wordTables.ts`, shared by terrain and grass | One owner for artifact layouts |

### Slice 02 (`ae645e7a`), by the implementing agent; reviewed by the orchestrator

| Decision | Why |
| --- | --- |
| A plate's `gpuMs` is null on Metal | Godot 4.7.2's Metal driver records no GPU timestamps, and a false 0 would lie. Perf was "record, don't gate", so nothing blocks. |
| `tests/run.sh` requires each test's result line, and the runner clears the environment | A GDScript parse error exits 0, and a failing `assert` hangs |
| The binding smoke moved to `native/godot/binding_smoke/` with `.gdignore` | Otherwise the imported project tries to load the binding's dylib on every run |
| `.gitignore` adds `*.uid` | Godot 4.7 writes them, and everything here loads by path |
| Building kits come from catalog `city_kit_*` keys; a unit with no catalog model becomes a reported capsule | Removes environment-driven selection and the hard-coded GLB fallback |
| Fog is drawn uncapped as one MultiMesh, only when the manifest lists it | Phase 1 menu manifests don't list it |
| The far plane stays at Godot's default 4000 m, reported as a budget, until slice 03 sets it | Camera work is slice 03 |
| The legacy layer's 1 km reach is measured from frame targets | Pre-parity code, replaced in slice 07 |
