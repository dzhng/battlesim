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
| Q0 | Scope | Phase 1 covers backend and capture, camera, lighting, terrain and grass (slices 01–06b). After phase 1 the user inserted native performance as phase 2 (07–12). Models, effects, composition and promotion became phase 3 (13–16). Slice numbers follow build order. | Same camera, light and ground first | User |
| Q1 | Who produces the control and manifest | One automated headless-browser run of `/graphics-test` at 1280×720 writes the capture, manifest and plates per scene | Reproducible; no manual clicking | User |
| Q2 | What a frame records | The resolved drawn view (`eye[3]`, `target[3]`, `up[3]` after clamping, clearance and lag), plus reel time, sim tick, interpolated sim time and canvas size. Wind reads the same interpolated sim time, so it has no separate field. The lens is recorded once per scene in the manifest. | Native ports no rig logic | User (base); agent added the clock and size fields from L2 under #3 |
| Q3 | Schema versions | None. Ids are `battle-presentation-capture`, `native-visual-presentation` and `godot-render-report`, changed in place. | The user doesn't read these files; only two ids had suffixes | User |
| Q4 | How the manifest grows | Per slice: each slice adds only what native reads | No unread fields | User |
| Q5 | Lighting during phase 1 | Lighting (04a–c) comes before terrain and grass | Judge colour, don't defer it | User |
| Q6 | Viewport and density | 1280×720 at 1× on both sides, always. This is what players see since `ace81f81`. | Simplest | User; confirmed by the territory |
| Q7 | Backend | Forward+ (Metal). Delete `gl_compatibility` and keep no fallback path. The slice 02 probe (one building, a grass multimesh, a shadowed sun, the sky shader, captured unattended) must pass; failure is a named blocker. | #2: shadows, AO and fog need it. #1: one backend. The probe already rendered. | Agent on the user's behalf (#1, #2) |
| Q8 | Phase 1 closeout | Run `review`, update the handoff, then plan phase 2 with a fresh walk. The full repo check runs once, at slice 16. | AGENTS.md: everything runs once at the end of the spec | Agent on the user's behalf (#1); territory |
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

At the phase 1 closeout, every landmine except L13 is built as decided. L13 (building tiers) belongs to slice 13. One partial: the legacy buildings-and-props layer still measures its 1 km reach from frame targets rather than from the eye (L4), until slice 13 replaces it.

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
| L13 | Building tiers depend on canvas pixels | `buildingTier.ts` (`ace81f81`) | Sharp edge, slice 13. The manifest records the chosen tier. |
| L14 | Slice 04a shadow cuts use unported buildings | Token-picked shells until slice 13 | Sharp edge, slice 04a. Judge shadow direction and softness, not shapes. |
| L15 | Small leaks | A material per unit per frame (`reel.gd:1159`); units reused by index rather than id (`:1165`) | Sharp edge, slice 02. Fix in passing. |

## Confirm before coding: resolved

- **The Forward+ probe on real market-town content passed.** `tests/forward_plus_probe.gd` renders a real building, the grass field, a shadowed sun and the sky with no blocker.
- **WebGPU canvas readback works without changing the shipped frame.** Only a plate-reading pilot configures the canvas with `COPY_SRC`; every other frame is configured as before, and a viewport test pins that.
- **Godot imports symlinked models when each file is linked separately.** A symlinked directory makes Godot write `.import` files into the real `assets/`, so staging links one file at a time.

## Phase 1 ledger

This replaces the walk's plan synthesis and the per-slice build logs. Every entry was checked against the code at the phase 1 closeout. Slice evidence lives in each slice's commit messages and in `throwaway/native-parity/`.

**Review these first:**
1. **Hash equality for grass, not bit-exact positions** (needs-user).
2. **Scar cells exempted by name from the grass count gate** (sound, medium confidence).
3. **The light enters the manifest raw, and native ports the derivations** (sound, medium confidence).

### Needs you

**Grass placement is exact in its integers, and within 16 ulps in its float positions.** *Slice 06a, `68a07659`.*
- **The plan said:** "grass sampling is ported bit-exact" (L1).
- **What happens:** a clump's identity, whether it grows, and which tile it belongs to come from integer hashes, and those match the browser exactly on 108 test candidates. Where it stands, a float position, comes out of two different shader compilers: Chrome's WGSL→Metal and Godot's GLSL→SPIR-V→Metal. They round differently, by up to 8 ulps (about 0.1 mm at 250 m).
- **The alternative:** force strict float maths in both clients, which neither compiler offers, or ship every clump position in the manifest, which is tens of megabytes per frame.
- **Reach:** any later slice that compares grass compares identity exactly and position within a tolerance.
- **Provisional call:** accept the ulp tolerance; it is invisible at any distance the camera reaches. To reverse it, change the test's 16-ulp bound to 0, and it will fail.
- **Confidence that you'd agree:** medium.

### Sound

Within each group, least confident first.

**Gates and evidence**

- **A scar cell is exempt from the 10% grass count gate only when native's excess is covered by its clumps on bare scars.** *06 closeout, `58931e34`.*
  - Picture a tank that crossed a wheat field earlier in the battle. The browser knows that side's learned scars (craters, scorch and tracks), so it leaves the grass out on them. Native has no scars layer until slice 14, so it grows grass there, and a screen cell over the track counts 13–16% more clumps than the browser.
  - The run replays the learned ground (`capture/learned_ground.gd`, tested against the browser's own replay), counts native clumps standing on bare scars per cell, and names a cell `scar-omission` only if the excess is no larger than that.
  - Every other cell must be within 10%. Five of 216 cells are named, and removing the allowance makes the run block.
  - Reach: slice 14 deletes the allowance when scars arrive.
  - Confidence: medium. The gate is unchanged, but it carries a named exception.

- **The terrain feature gate is a measured class agreement, not a judgement by eye.** *05a, `48c3ab7a`.*
  - Both clients drew the same frame as bytes: is this pixel paving, water, forest, or plot kind K? The two were compared over ground both drew, away from edges.
  - It came out at 100.00% on both maps, against a 99.5% gate. A one-digit hash change drops plot agreement to 18%, so the measure can fail.
  - The native class branch and the browser's parity class plates were removed at the closeout, as the plan said. A future terrain regression check would need them back.
  - Confidence: medium. It's a strong check that now exists only in history.

- **Godot's Metal GPU time is null rather than measured.** *Slice 02.*
  - Godot 4.7.2 records no GPU timestamps on Metal, so the report writes null rather than a false 0.
  - Performance was "record, don't gate", so nothing blocks. Phase 2 measures it with Metal System Trace (slice 07).
  - Confidence: high.

- **Native renders at the browser's 4× MSAA, and post reads the resolved image.** *`11404057`.*
  - The browser shades, hazes and resolves 4 samples per pixel before post; native now does the same.
  - Grass MAE against the browser fell from 7.04 to 0.76 on one plate.
  - One difference remains: native hazes the resolved pixel, because Godot's compositor sees only resolved buffers. No halo shows.
  - Confidence: high.

**Contracts and formats**

- **The manifest carries the raw `presentation.light` plus `sunDirection`; native ports every derivation.** *04a.*
  - The derivations are the sun colour, overcast, extinction, cascade splits, sky radiance, haze and post. The alternative was for the browser to export resolved values (`light.sun`, `light.post` and so on).
  - Native refuses a missing or extra field and a non-unit sun direction, and a headless test checks each ported derivation against values computed by the TS code.
  - Reach: a browser change to the light formulas needs a native port; a change to the light data needs nothing.
  - Confidence: medium. It duplicates formulas by design (L1), and the tests catch drift.

- **The browser's packed terrain and grass buffers ship as hashed artifacts, and their layouts are generated from the GPU struct definitions.** *`5d6898ad`, `b726a9ca`.*
  - Native reads exactly the words the browser uploaded, rather than porting about 2,300 lines of plot, biome and surface logic. A struct change regenerates the layout and can't drift.
  - Reach: these binary layouts are now a contract between the clients.
  - Confidence: high.

- **Tables reach the GPU as RGBA8 textures, one texel per word.** *05a.*
  - Godot 4.7.2's `Texture2DRD` refuses `R32_UINT`, which the plan named.
  - The shader reassembles each word exactly, and a probe confirmed every byte round-trips.
  - Confidence: high.

- **Terrain layout offsets and tuning reach the shader as generated defines, not uniforms.** *05a/05b.*
  - The browser splices the same numbers into WGSL. A renamed field fails to compile, and that failure is a logged blocker.
  - The plan said "uniforms".
  - Confidence: high.

- **The light and the tuning tables are passed in by the page, and the Node manifest writer checks only their shape.** *`449e9d8e`, `948dc7fd`.*
  - The scene harness runs the writer under plain Node, which can't import the renderer.
  - The page validates the light with the renderer's `validateLight` and derives `sunDirection` with `sunDirection()`. Native re-checks strictly.
  - Confidence: high.

- **No separate wind clock; `samples[].camera` dropped.** *Slice 01.*
  - Grass wind reads the interpolated sim time, which the frame record already holds. The per-sample camera had no reader.
  - Confidence: high.

- **A `?scenes=` subset run reports its own workload.** *Browser review.*
  - It used to embed the whole reel's workload, so a partial capture could pass for the complete one. Found by the Codex review.
  - Confidence: high.

**Rendering choices that make native match**

- **Shadow softness is PCF matched in the second cascade, and Godot's shadow biases stay at default.** *04a.*
  - Godot has one blur radius per light; the browser has a fixed penumbra in metres. So native's near shadows are slightly sharper and its far shadows softer.
  - Confidence: medium.

- **The sky's radiance is evaluated per pixel instead of baking the browser's 384×192 lookup texture.** *04b.*
  - No bake pass or texture lifetime to manage. The only difference is the browser's bilinear filtering.
  - Confidence: high.

- **The post chain is a formula port: bloom, grade and AgX as one CompositorEffect, with Godot's tonemap linear.** *04c.*
  - Godot's built-in AgX can't express the grade's split tone and lift.
  - Mean brightness is within 1–4 levels of the browser's.
  - Confidence: high.

- **Sun energy is `sun_intensity / π`, its colour is converted to sRGB, and shadow opacity is `1 − shadow_floor`.** *04a.*
  - These are Godot's unit conventions mapped to the browser's.
  - Confidence: high.

- **The grass build is a compute shader writing the MultiMesh and its indirect draw count.** *06a.*
  - The 4.4-era bug (godotengine/godot#105100) didn't reproduce on 4.7.2 once each MultiMesh had a custom bounding box. Godot leaks the indirect command buffer at exit (godotengine/godot#108148), which shows as a 2-RID warning.
  - Confidence: high.

- **WGSL-exact `smoothstep` and `roundEven` live in one shared include.** *05a, closeout.*
  - GLSL leaves both undefined at their edges.
  - Confidence: high.

- **Units draw at their authored size; the legacy 1.8× scale is gone.** *03.*
  - It doubled every vehicle and misframed subjects.
  - Confidence: high.

**Tooling and project shape**

- **A logged GDScript error becomes a blocker, and tests fail on any logged error.** *03.*
  - GDScript runtime errors return null and carry on, so a broken model load looked like success. This exposed two bugs.
  - Confidence: high.

- **Tests call `quit(code)` rather than `assert`, and `tests/run.sh` requires each test's result line.** *02.*
  - A failing `assert` hangs script-mode Godot, and a parse error exits 0.
  - Confidence: high.

- **One owner per native concept in `core/`.** *Closeout, `bcaa8fff`.*
  - Strict JSON checks, shader number text, compute-pass and word-table helpers, and the shader includes for word reading, coordinates and smoothstep. Parallel builders had copied each of these.
  - Confidence: high.

- **Plates come from the existing `/graphics-test` route in plate mode.** *01, browser review.*
  - Only a plate-reading pilot makes the canvas copyable, so the shipped game is configured as before.
  - The alternative was a separate route duplicating the reel page.
  - Confidence: high.

- **The test fixture is a trimmed real paris-corner capture (580 KB, artifacts included) with a frozen map copy.** *03.*
  - Confidence: high.

**Delegated discretion,** not audited: PNG encoding, plate-frame choice (each shot's middle), module layout below the named files, and contact-sheet layout.

### Unsound

None found at the closeout. Two earlier choices were corrected during the build and are already fixed:
- the manifest writer importing the renderer under Node (`8209a90b`, fixed by `449e9d8e`);
- the first light order, tone before sun and sky (fixed in the spec before any light code).

### Known gaps carried into phase 2

- **Infantry.** Only the own-side unit group is decoded, one model per unit, so squad members, enemies and corpses are missing, and one soldier is T-posed. Slice 13.
- **Missing models.** Tree lines and forests are drawn as flat bands, there are no wrecks, Paris balcony railings are missing, and lamp posts are 15–20% short. Slice 13.
- **Scars, decals, dust, smoke and fire.** Slice 14. This also removes the grass gate's scar allowance.
- **Unexercised paths.** The water formulas are ported but unmeasured, because neither reel map has water. Relief is tested only against a synthetic grid.
- **Run-to-run noise.** Native grass plates differ by one or two pixels between reruns, probably GPU append order. Byte comparisons need that floor.
- **Git history.** The two closeout reviewers shared one git index, so their commits were rewritten into `ed268e8b`, `bcaa8fff` and `ef730416`, with a tree identical to the pre-rewrite HEAD. Nothing had been pushed.

## Phase 2

### Slice 11: release build

- **The release export runs the parity run unchanged, through one script.** `native/godot/release.sh` exports the `macOS release` preset into `throwaway/native-release/` and passes every argument through, so plates and `--bench` both run from the exported app.
  - It re-exports on every call (10–20 s) rather than tracking staleness.
  - Confidence: high.
- **An export packs `staging/` as it was at export time; an asset it lacks is a named blocker.** An export can neither link nor import, so `Staging.stage` only looks up packed resources there. An editor run of a scene stages its assets.
  - The alternatives were runtime glTF loading (a second loading path whose import settings could differ from the editor's) or a headless staging pass that duplicates the run's admission. Both go away or change when slice 13 replaces the legacy map layer.
  - Confidence: medium. A fresh checkout needs one editor run per scene before its first export.
- **The exported app takes the repository root from `BATTLEGAME_REPO`.** The script sets it. The editor still derives the root from `res://`. Deriving it from the executable's path would break under a symlinked `throwaway/`.
  - Confidence: high.
- **The preset is universal, and the project imports ETC2/ASTC beside S3TC.** Godot's 4.7.2 templates ship only universal binaries, and an arm64 or universal export refuses to run without ETC2/ASTC imports. The editor's plates did not change with the setting (both scenes byte-identical but one market-town pixel, 3 levels).
  - Confidence: high.
- **Signing is Godot's built-in ad-hoc signature; there is no notarisation.** Apple Silicon needs some signature to run the binary.
  - Confidence: high.
- **The report says what ran it:** `renderer.build` is `editor`, `debug` or `release`.
  - Confidence: high.
- **Templates are installed by hand,** from the official `.tpz`, checked against the release's SHA512-SUMS, with only `macos.zip` and `version.txt` copied into the user's Godot data folder.
  - Confidence: high.
