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
| Q8 | Phase 1 closeout | Run `review`, update the handoff, then plan phase 2 with a fresh walk. The full repo check runs once, at slice 18. | AGENTS.md: everything runs once at the end of the spec | Agent on the user's behalf (#1); territory |
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

At the phase 1 closeout, every landmine except L13 is built as decided. L13 (building tiers) belongs to slice 15. One partial: the legacy buildings-and-props layer still measures its 1 km reach from frame targets rather than from the eye (L4), until slice 15 replaces it.

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
| L13 | Building tiers depend on canvas pixels | `buildingTier.ts` (`ace81f81`) | Sharp edge, slice 15. The manifest records the chosen tier. |
| L14 | Slice 04a shadow cuts use unported buildings | Token-picked shells until slice 15 | Sharp edge, slice 04a. Judge shadow direction and softness, not shapes. |
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
  - Picture a tank that crossed a wheat field earlier in the battle. The browser knows that side's learned scars (craters, scorch and tracks), so it leaves the grass out on them. Native has no scars layer until slice 16, so it grows grass there, and a screen cell over the track counts 13–16% more clumps than the browser.
  - The run replays the learned ground (`capture/learned_ground.gd`, tested against the browser's own replay), counts native clumps standing on bare scars per cell, and names a cell `scar-omission` only if the excess is no larger than that.
  - Every other cell must be within 10%. Five of 216 cells are named, and removing the allowance makes the run block.
  - Reach: slice 16 deletes the allowance when scars arrive.
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

- **The sky's radiance was evaluated per pixel instead of baking the browser's 384×192 lookup texture.** *04b; reversed in slice 10.*
  - Slice 10 measured it: the sky and the haze (which integrated it twice per pixel) cost 2 ms a frame where the browser's lookup costs almost nothing. Native now bakes the browser's lookup.

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

- **Infantry.** Only the own-side unit group is decoded, one model per unit, so squad members, enemies and corpses are missing, and one soldier is T-posed. Slice 14 (units) or 15 (models and scenery).
- **Missing models.** Tree lines and forests are drawn as flat bands, there are no wrecks, Paris balcony railings are missing, and lamp posts are 15–20% short. Slice 14 (units) or 15 (models and scenery).
- **Scars, decals, dust, smoke and fire.** Slices 16 (scars) and 17 (effects). The scars slice also removes the grass gate's scar allowance.
- **Unexercised paths.** The water formulas are ported but unmeasured, because neither reel map has water. Relief is tested only against a synthetic grid.
- **Run-to-run noise.** Native grass plates differ by one or two pixels between reruns, probably GPU append order. Byte comparisons need that floor.
- **Git history.** The two closeout reviewers shared one git index, so their commits were rewritten into `ed268e8b`, `bcaa8fff` and `ef730416`, with a tree identical to the pre-rewrite HEAD. Nothing had been pushed.

## Phase 2

### Slice 07: profiling harness

Commits `2fd549f2b`, `cfe6887ae`, `39850b7b7`. Closed by the coordinator on the user's behalf where marked; otherwise by the agent or the territory.

- **One summariser, raw frames from both clients.** Native writes raw per-frame intervals and CPU times; `frameComparison.ts` (`web/fps-compare.mjs`) computes every number for both with the browser's `frameTimeSummary`. Only the interval clock differs: rAF timestamps against `Time.get_ticks_usec`.
  - Confidence: high.
- **The comparison pair is minimised native against the headless browser; neither presents to the display.** A minimised window skips presentation: the same 300 market-town records took 25–26 ms a frame minimised and 37–48 ms shown. Presenting is reported as its own cost, from a shown sample. *Coordinator, for the user.*
  - Confidence: high.
- **Both clients are compared without uncapping the browser.** Chromium's `--disable-frame-rate-limit` hangs the graphics test in battle preparation: the battle frame is built once, then the model layer's `install` never finishes (time-boxed, root cause not found). `--disable-gpu-vsync` alone leaves rAF at the display's 120 Hz. The browser stays paced and native runs uncapped. The cap doesn't bind while the browser's frames take over 8.3 ms. *Territory; pairing rule from the coordinator.*
  - Confidence: medium. A browser that drops under 8.3 ms a frame would need CPU and GPU time per frame instead.
- **Every comparison runs plugged in.** On battery the headless browser holds exactly 30.0 FPS (every interval 33.3 ms) on frames that cost 7 ms of CPU and 9 ms of GPU; on AC the same reel runs at 50–61 FPS on market-town. A trivial WebGPU page reaches 120 Hz on battery, so it isn't Chromium's battery saver. The cause is not found.
  - Confidence: high that it's the power source; unknown why.
- **Native draws every frame itself.** Godot's main loop skips drawing, and sleeps, while its window is minimised or covered (`can_any_window_draw`), so runs that awaited `frame_post_draw` stalled for as long as the window stayed hidden. One stall was reproduced for 18 minutes; `sample` showed the main thread in `nanosleep` in 98% of samples. The loops now `force_draw` with the render loop off, after one main-loop iteration per record, which pushes moved nodes' transforms. Force-drawn plates are byte-identical to the old loop's. A shipped client pausing while hidden is normal game behaviour, a phase 3 question.
  - Confidence: high.
- **The window starts minimised.** macOS clamps `--window-position 10000,10000` back on screen (to 872, 592), so every earlier "off-screen" run was visible.
  - Confidence: high.
- **GPU traces give proportions and gaps, never absolute ms.** Metal System Trace slows a native frame about 2.7× (a 69 ms span traced against 26 ms untraced). Untraced runs give the ms. *Coordinator, for the user.*
  - Confidence: high.
- **A trace is discarded above 10% foreign GPU work.** The T3 Code app the user watches does 3–6% of the GPU's work during traces; that is ambient and hits both clients alike. Every trace reports its share. *Coordinator, for the user.*
  - Confidence: medium.
- **Encoders are grouped by channel and command index, not by pass label.** Godot 4.7.2's `RenderingDevice` debug labels don't reach Metal's encoder labels; native's compute passes carry them anyway. Passes are attributed by ablation (slice 08).
  - Confidence: high.
- **The Metal HUD log is no source.** On this macOS its switch is `MTL_HUD_LOG_ENABLED` (the 2022 talk's `MTL_HUD_LOGGING_ENABLED` is stale), and its per-frame values are redacted as `<private>` without `sudo log config`.
  - Confidence: high.
- **The graphics test shows the in-game frame-rate counter** (a pilot's `frameRate`), so its readings can be checked against the recording. They agree within 2% on every run so far.
  - Confidence: high.
- **Both sides refuse to time while the other client draws,** matching the other client by its executable path or process name; a command line that merely mentions it doesn't count.
  - Confidence: high.
- **Not done: the CPU profile.** No Time Profiler capture was taken. Native CPU time per frame is 6–7 ms against 26 ms frames, so the CPU isn't the bottleneck.

### Slice 09: native model drawing architecture

Closed by the agent on the evidence; the user had not reviewed it at the time of writing.

- **One MultiMesh per appearance and tier, its mesh one surface per material.** Each tier's parts are merged with their transforms baked in (`SurfaceTool`), so a 1,247-part Abrams draws one surface per material a pass (the reel's units average about 15), not one per part. The slice's "one MultiMesh per appearance part" became per appearance, because parts would have kept draw calls in the thousands.
  - Confidence: high.
- **The authored `_LOD<n>` names are the tiers, and Godot's automatic mesh LODs are off** (`rendering/mesh_lod/lod_change/threshold_pixels = 0`). The legacy layer drew all four authored tiers of every model on top of each other, the main source of 08's primitives and of z-fighting speckle. One level-of-detail owner, as in the browser; merged meshes have no Godot LODs anyway.
  - Confidence: high.
- **Tiers follow the browser's rules, at chunk granularity for statics.** Models by projected height (`presentation.models.lod_px`), buildings by a metre's pixels (`presentation.buildings.lod_px_per_m`), statics in 64 m chunks tiered at the chunk's nearest point, movers one by one. The browser tiers props per instance inside near chunks; the chunk rule is never coarser than that, and a chunk's batches are rewritten only when the chunk crosses a tier boundary or the view's edge.
  - Confidence: high.
- **The detail thresholds are read from `fixtures/game.json` by the legacy layer,** like the catalogs it already reads, not from the manifest. Slice 13 moves them into the manifest with the rest of placement.
  - Confidence: medium. It is a second reader of `presentation`, with a named removal.
- **Shadows mirror the browser, not the brief's wording.** A mesh-drawn model casts through a shadow-only batch one tier coarser (`CASTER_COARSER`), the coarsest tier casts as itself, and view batches cast nothing. A chunk out of view whose shadow can land in it casts at the coarsest tier. The brief said far tiers don't cast; the browser's far props do, out to the cascades' 2.6 km, and dropping them would move native away from it.
  - Confidence: high.
- **Kit shells draw every tier at once.** A shell's finer tiers have openings that facade modules fill, and the legacy layer places no modules, so a single tier showed hollow Paris blocks. Shells are about 1% of draws; slice 15 replaces them.
  - Confidence: high.
- **Skinned parts are posed on the CPU at build** (bind poses times the skeleton's pose), because a MultiMesh draws no skeleton. Without it the soldiers shifted by a few pixels.
  - Confidence: high.
- **No impostor cards yet.** The browser bakes cards only for skinned soldiers, from their far pose. Native has six unposed soldiers in the reel, whose coarsest tier is 755 triangles, and the window runs at 8.5 ms against a 16.7 ms budget. Cards baked from an unposed soldier would show the wrong pose, so they wait for slice 14's articulation. *Agent, against the slice's contract; spending margin on simplicity.*
  - Confidence: medium.
- **The runs use the assets imported on 2026-10-10.** A merge of main during the slice changed the Abrams and both soldier sources; the staging only imports new files, so native kept the earlier imports, which match the browser control. A fresh import (or a new control) brings in main's models.
  - Confidence: high.

### Slice 10: per-pass budgets and presentation

Closed by the agent on the evidence; the user had not reviewed it at the time of writing.

- **A new control, `throwaway/native-parity/2026-10-11T08-56-34-958Z`, with a fresh native import.** Main's merge changed the tank and soldier models; slices 10 and 12 compare against this run. The 10 October control and imports are retired.
  - Confidence: high.
- **A kit shell's open tier draws over every coarser tier; its closed tiers draw alone** (`closed_tier: 2` in the legacy layer's building detail, from `OPEN_TIERS = (0, 1)` in `kit.py`). Drawing the chosen tier plus tier 2 left Paris dormers hollow, and the chosen tier plus tiers 1–2 lost a balcony's front: the coarser tiers each fold in part of what the modules supply. Near buildings look as before, and a far one draws one tier instead of four. Slice 13 places the modules and replaces the rule.
  - Confidence: medium. It is scaffolding read from the kit's build script, not from a manifest field.
- **Far props keep their coarsest tier out to the horizon.** The browser does the same and has no prop impostors; once the shells were fixed, props were not in the tail.
  - Confidence: high.
- **Native bakes the browser's sky lookup once and samples it** in the shown sky, the sky's light and the haze (`environment/sky_lut.gd`). It replaces 04b's per-pixel sky: GPU busy fell from 6.31 to 4.43 ms a frame, and plates moved at most 4 levels, neither toward nor away from the browser.
  - Confidence: high.
- **Per-pass costs come from traces of both clients on the same stretch of reel.** The browser's are its labelled encoders, with Dawn's `use_user_defined_labels_in_backend` turned on in a scratch copy of the scene runner's flags for the trace only. Native's are busy time saved by ablation over 100 looped records. The flag isn't committed: traces are occasional evidence, and the scene runner's flags are the shipped browser's.
  - Confidence: medium. Ablation rows overlap, and single traces carry about ±0.3 ms.
- **No presentation setting changed.** On screen and off differ by about 0.1 ms now, and swapchain count and frame queue size moved nothing beyond the spread. 08's +17 ms came with a saturated GPU.
  - Confidence: medium. Each run's on-screen state was read from the window server, because macOS restores the last window state whatever the project or `--windowed` asks.

### Slice 11: release build

- **The release export runs the parity run unchanged, through one script.** `native/godot/release.sh` exports the `macOS release` preset into `throwaway/native-release/` and passes every argument through, so plates and `--bench` both run from the exported app.
  - It re-exports on every call (10–20 s) rather than tracking staleness.
  - Confidence: high.
- **An export packs `staging/` as it was at export time; an asset it lacks is a named blocker.** An export can neither link nor import, so `Staging.stage` only looks up packed resources there. An editor run of a scene stages its assets.
  - The alternatives were runtime glTF loading (a second loading path whose import settings could differ from the editor's) or a headless staging pass that duplicates the run's admission. Both go away or change when slice 15 replaces the legacy map layer.
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
- **First debug against release reading, market-town, one run each.** 1,000 records (385–1384), 60 warm-up frames, uncapped, minimised window, plugged in, M5 Pro. This is a single sample, not a gate; slice 12 takes three runs with the spread shown.

  | Build | Average FPS | Frame p50 | Frame p95 | Frame max | CPU p50 | CPU p95 |
  | --- | --- | --- | --- | --- | --- | --- |
  | Editor (debug) | 15.5 | 62.8 ms | 77.0 ms | 94.8 ms | 27.0 ms | 38.6 ms |
  | Release export | 17.7 | 54.5 ms | 64.8 ms | 75.6 ms | 12.5 ms | 26.3 ms |

  - Release halves the CPU time per frame, but frames stay around 55 ms. Both draw the same counts (about 25k draw calls and 24.5M primitives per frame). The bottleneck is not the debug build; it is what slice 08 is naming.

### Slice 12: performance gate

Closed by the agent on the evidence; the user had not reviewed it at the time of writing. Evidence is in `throwaway/perf/gate/`.

- **The gate's figures come from native uncapped, minimised, against the headless browser,** the pairing slice 07 chose.
  - A second set with native capped at the display (`--pace display`) gave the same picture: both clients are paced by the display either way. It is kept as evidence and is not the gate.
  - Confidence: high.
- **The bench minimises its window again just before timing** (`parity_run.gd`). The project starts minimised, but in 4 of 6 runs of each set macOS had the window back on screen. Every on-screen run read paris-corner p50 8.37 ms, against the browser's 8.34, and every minimised run 8.22–8.33.
  - Re-minimising held 12 of 12 runs minimised (6 probes, 6 gate runs), and the measure passed.
  - It enforces the stated condition (no presentation, as the headless browser) rather than changing the bar or the clock.
  - The other fix considered was a vsync-aligned native clock, matching rAF's. Godot exposes no present timestamp, so it was not contained.
  - Confidence: medium. Why macOS restores the window is not known; the report's `window` field is the check.
- **GPU busy per frame is a proportion, calibrated.** Each trace gives the client's GPU busy share of wall time: the union of its top-level encoders over its first to last GPU work. That share times the untraced mean frame interval gives ms per frame; each figure is the median of three traces.
  - Wall-time share sidesteps Metal frame grouping, which differs between the clients. The browser's headless frames don't map to Metal frames, and `gpu-trace.mjs` spans can be infinite there.
  - The browser's timestamp queries show the calibration overstates the traced browser by 14–39%. Both figures are reported.
  - Confidence: medium.
- **The traces are separate runs from the timing runs,** at about a quarter, half and three quarters of each scene. Tracing slows frames, so a traced run is never a timing run.
  - The native traces predate the window fix, and four of the six were on screen. Presenting adds native work, so the comparison errs against native; they were not re-taken.
  - Confidence: high.
- **CPU ms is reported, not gated.** On paris-corner native's CPU p50 is 1.54 ms against the browser's 0.88. The two clocks cover different work: the browser's simulation runs in a worker and its GPU submission in Chrome's GPU process. Slice 12's measures are frame time, FPS and GPU time.
  - Confidence: medium.

## Phase 3 plan (2026-10-11), by the orchestrator on the user's behalf

The spec said phase 3 would be planned with the user. Under the implementation goal, the orchestrator planned it from the confirmed preferences (#1 no unasked machinery, #2 don't defer fidelity, #3 the browser resolves and native draws, #4 automate evidence). Each call is reversible.

| Decision | Why | Preference |
| --- | --- | --- |
| No new simulation-to-capture data path. Effects read causes the capture already carries. | A source map showed every effect cause is in the publication: projectiles, blasts, shot counters, smoke, crashes, `knownProps`, `fallenBodies`, and the ground's scars and `cleared` byte. The only gap was native decoding one group. | #1 |
| The browser exports static per-map placement (building modules and states, scenery); native ports per-frame derivations (poses, effects, fallen, felled and cleared state) | Placement is a pure function of map plus catalogs, as terrain tables were. A live native client must derive per-frame state itself, so porting it now isn't wasted. | #3, endgame b |
| Each content slice gates on per-plate counts equal to the browser's, plus seeded or vector tests, then the visual gates | The same pattern as grass counts in 06a: load-free, exact, and able to fail | #4 |
| Each content slice re-runs slice 12's performance bar | Content adds cost, and native must stay no slower than the browser | User (phase 2) |
| Phase 3 is renumbered 13–18 | Units, buildings and scenery, water and scars, and effects are separate visual variables | — |

## Phase 3 build log

### Slice 13: full publication decode

| Decision | Why | Closed by |
| --- | --- | --- |
| The decoder is its own module, `capture/observation.gd`, named for the browser's `observation.ts`; `presentation_capture.gd`'s `decode_samples` drives one stream through it | One owner for decoding a publication; the capture parser stays a parser | Agent (delegated: native's record shapes) |
| A sample's record is the browser's `ObservationView` exactly: the same keys and nulls, `own[].kind` a name, the ground as the browser's `groundPatch` runs | The fixture compares by deep equality, and later slices port browser code that reads these names | Agent (#3) |
| Numbers compare at float32 precision; the fixture writes each as its shortest float32 text, and its null stands for the browser's NaN | Both decoders read float32 carriers; JSON holds no NaN | Agent |
| The fog words compare by SHA-256 digest | The market-town field is 17,579 words a sample | Agent |
| The market-town fixture is samples 1824–1830 of control `2026-10-11T08-56-34-958Z`, the first re-encoded as a fresh record (snapshot groups, its full fog, an empty learned ground) by a scratch script | A delta needs its baseline, so a window can't be cut out verbatim; the empty ground keeps it at 155 KB (262 KB with the full learned ground). Both decoders decode the same words, so the comparison is unaffected | Agent |
| Not covered by either fixture: guided missiles, crashes, member leans, encounter, skirmish (objectives, pending purchases). They decode through the same row and section code, but no field is proven against the browser | No menu-reel capture publishes them. The slice that first draws one (17 for crashes and guided) adds a short stream from the WASM battle | Agent; open |
| Stricter than the browser: a stale record, a word index outside its table, or a layout without a field or section native reads is a blocker, as is an unknown group | A capture is one ordered stream from a known layout; the browser skips stale records and reads missing fields as undefined | Agent (#2) |
| The learned ground keeps the cleared byte (`cleared_at`), with no drawing consumer yet | Slice 15 (felled forest) and 16 (scars) read it | Agent |
| Decoding every group costs startup: the full market-town capture decodes in 4.1 s, up from 1.2 s | Within the 30 s startup budget | Agent (user's startup budget) |
| The paris-corner decode fixture is 888 KB (43 KB gzipped) | Every sample of the capture, as the gate asks | Agent |

### Slice 15: buildings and scenery, browser half

| Decision | Why | Closed by |
| --- | --- | --- |
| Building and scenery placement rides `UploadedTables` as two more groups (`buildings`, `scenery`), written as `records` artifacts beside the terrain's and grass's; no new layout kind | One export path and one artifact contract; the record layouts come from `d.struct`s through `fillRecords`, so a field change regenerates the layout | Agent (#1) |
| Buildings are expanded per building and state: tables `intact`, `ruin`, `gutted`, each building's rows in template order, plus a `buildings` table (frame, intact bounds, owner, chunk, row count per state, -1 where the template has none) | Native swaps a building's state by its index without placing anything; a missing state stays refusable by name | Agent (#3) |
| A module instance names a compact module list (only the modules the map's templates place, in library order), not the library's thousands | Keeps the manifest and the per-plate counts small | Agent |
| A building's chunk is the browser's coarse-chunk index; native forms chunk boxes as the union of its buildings' exported bounds (the fallen population too) | The browser's chunk box is exactly that union, and the bounds need the installed kits' mesh bounds, which only the browser holds | Agent |
| Scenery exports the whole placement (forest with trunk ids, understorey, backdrop, and every dressing cell laid whole, with a cell table); felled, cleared and fallen state stay per frame | Placement is static; the dressing's cell order matters to its pool, so cells travel as cells | Agent (#3) |
| The tier rules are numbers in the manifest (`buildings.tiers`, `scenery.tiers`, including pool and per-view budgets); the formulas are native's to port | Same split as the light: data exported, formulas ported | Agent (phase 1 L1 pattern) |
| Per-plate counts (`instanceCounts`) count what each layer draws into view, not its draw ranges: hidden coarse records and ranges bridged across chunks out of view are left out; dressing on cleared ground (laid with no size) is left out | Native should match what is seen, not the browser's batching | Agent |
| Counts are per kit module and tier, per scenery population, appearance and tier, stumps per appearance, and known props (wrecks, shoved bodies, rubble) per appearance at each tier then as a card | A tier mismatch shows as a count mismatch, not just a missing kind; known props are slice 15's (coordinator) | Agent; coordinator for known props |
| Counts are taken before the plate's grass-less redraw | That redraw may lay more dressing or expand more buildings | Territory |
| Not exported: a building's part props and the gutted-shell prop types, which native needs to derive fallen state from `knownProps` | They are per-frame derivation inputs, not placement; the native half decides whether they come from the map export or the manifest | Agent; open |
| New control `throwaway/native-parity/2026-10-11T12-20-57-541Z` (`3f14bf2c9`). It ran with slice 16's uncommitted terrain edits in the shared tree, so its terrain group also lists `scars` and `water`; everything slice 15 reads came from `3f14bf2c9`. Market-town's placement artifacts total about 90 MB, the dressing 43 MB of it | Plate-mode scratch only; costs nothing outside plate mode | Agent (spend margin on simplicity) |

### Slice 16: water surface and ground scars

| Decision | Why | Closed by |
| --- | --- | --- |
| The scar look reaches native as the terrain table `scars`, the browser's `ScarParams` record as last written; native reads its grid and look, and ignores its page-cache fields (`pages.zw`, `clip`, `cacheBounds`, `defaultWord`) | One export path (terrain tables); the look is biome data the browser already packs, so native re-reads no fixture | Agent (#3) |
| Native's scar upload is one word table: normalisation words, a directory word per 16-cell tile, a page per marked tile, rewritten per changed tile and uploaded whole; the browser's view-sized hashed page cache is not ported | A market-town battle marks about 450 tiles (16.5k cells), so the whole learned ground fits in about 1 MB; values are byte-identical, filtered with the browser's exact RGBA8 arithmetic | Agent (delegated: the upload path; spend margin on simplicity) |
| The learned ground is replayed at every record, before the draw, not only at plates; `bare_at` is replaced by `marks` | The ground and grass now draw scars every frame, and the allowance that read `bare_at` is gone | Agent |
| The water surface's cells travel as the terrain table `water` (corner, height, side per cell), the cells `waterSurfaceMesh` lays (`waterCells`) | The browser resolves, native draws; the surface height isn't in the surface records | Agent (#3) |
| Water is judged by vectors only: `waterSurface` and `waterNormal`, resolved by TypeGPU and run on WebGPU, with `groundWater`'s depth fed in; 210 points equal | The capture scene films only the menu backdrop's `menu` maps; `river` is a `test` map with no shots, and filming it would need game data or a new workload nobody asked for | Agent (#1); the slice allowed it |
| Native water in shadow keeps Godot's own light, not the browser's `water.shadow` share of it; reported as the budget row `water.shadow` | Scaling lit water in Godot needs a custom `light()`, which drops its ambient; no reel map has water to judge it on | Agent; open until a map with water is filmed |
| Slice 16's evidence uses slice 15's control `2026-10-11T12-20-57-541Z` through a scratch copy of its manifests without `buildings`, `scenery` and `instanceCounts` | The control already carried `scars` and `water`; native's reader didn't admit slice 15's keys yet | Agent |
| Scars cost about 1.4 ms at p95 (market-town release, 3 interleaved runs each); traced GPU busy doesn't resolve the difference | Within slice 12's bar by a wide margin (browser p95 23.9) | Agent |

### Slice 14: units, poses, corpses and aircraft

| Decision | Why | Closed by |
| --- | --- | --- |
| Wrecks are known props, slice 15's; slice 14 draws soldiers, vehicles, corpses and aircraft, and counts those | A wreck is drawn by the browser's props path (`propAppearance.ts`), beside shoved cars and rubble, not by its pose feed | Orchestrator |
| The browser resolves what is static and exports it in `units`: each fielded type's facts, the appearance sets `AppearanceCatalog.resolve` picks from per side (in the side's faction look), each appearance's joints and clips or nodes and its computed rig, the pose feel, the lean hold and the side tints. Native ports the id pick and every per-frame derivation | Same split as the light and the placement: data exported, formulas ported. A live native client needs the derivations anyway | Agent (#3) |
| The dense clip tables (`buildClipTable`) are exported as artifacts, one per distinct table; market-town's 27 bodies share 2 of 1.44 MB | Native samples exactly the floats the browser's pose kernel reads, with no port of the clip quantisation or the bind fallback | Agent (#3) |
| `modelCounts` count the browser's pose frame (resolved soldiers, ground vehicles, aircraft, lying and fading corpses), not what is drawn into view | Exact and load-free like the grass counts, and it checks the population logic (deaths, the dying clip's end, the corpse cap). What is in view depends on culling, which both sides port separately | Agent (#4) |
| Native poses every frame record in order, including the records a plate run skips, from the capture's first frame record | Poses carry state from frame to frame (a walker's phase, an eased gun, a fall). The browser's frames before its recording starts are not recorded, so a pose that began during warm-up can differ slightly; counts equal on every plate regardless | Agent |
| A record's publication is the sample at its tick, blended from the one before by the record's `simTime` (`simTime·tickHz − previous tick`) | A record's `simTime` always lies within its last tick (checked over 3,964 paris records), and it is the browser's presentation clock exactly | Territory |
| Positions in the ported derivations are double-precision `[x, y, z]` arrays, not `Vector3` | Float32 positions moved a walker's stride-driven phase by up to 0.03 over a few hundred frames | Agent |
| Soldiers keep the glTF's own vertices and skin; each skin bind moves with the bundle joint of its name (or its nearest named ancestor), and the kernel writes one palette matrix per bind: `C · world[joint] · bind` | Mesh building stays native speed (about 0.1 s a body, against several seconds for CPU-skinning each tier at rest), and a bind's matrix folds the glTF-to-bundle mapping once. The build refuses a glTF whose joints stand more than 2 cm from the bundle's | Agent (delegated: sampling on GPU or CPU) |
| The browser's pose kernel is ported to a compute shader writing an RGBA32F palette image (`Texture2DRD`) that the model surface reads by instance (palette base in the tint's alpha) | GDScript would cost about 0.4 ms per soldier for 53 joints; spatial shaders cannot read storage buffers | Agent |
| Vehicles are posed on the CPU into a second palette (`ImageTexture`), one matrix per articulated node; a part follows its nearest articulated ancestor. A track's texture scrolls by the palette's first texel | A handful of vehicles and nodes a frame; the browser poses articulated rigs on the CPU too | Agent |
| Corpses are the living body's skinned mesh at its corpse pose, one palette per body and way of lying, not a baked static mesh | No second set of meshes or CPU skinning at build; it looks the same | Agent |
| Detail sizes (tier, culling) come from the coarsest tier posed on the CPU, not the finest | Tier 3 is about 2,000 vertices against 46,000. Its box is the same to a few centimetres | Agent |
| Model batches no longer pose skinned meshes; a skinned mesh outside the units layer is an error | The units layer is the one owner of skinning | Agent (refactor-clean) |
| Not ported: impostor cards. A model under `impostor_px` draws its coarsest tier, counted per run (`units.impostors`, 279 over market-town's plates) | A card replaces a mesh under 10 px tall: the same few pixels and colours. The frame has margin, and the bake (a SubViewport atlas of 16 views per body, relit) is a subsystem of its own. Against the slice's contract | Agent; open |
| Not ported: the side-tint mask (ORM alpha); a material's tint share applies uniformly | Slice 15's model surface does the same for every model; blue's tint is white and red's warm | Agent; open |
| The fixture run's manifest carries `units` written by `nativePoseVectors.test.ts`, with its 1.44 MB clip table beside it | The fixture run draws its units, and the pose test reads the same section | Agent |

### Slice 15: buildings and scenery, native half

| Decision | Why | Closed by |
| --- | --- | --- |
| Native ports only per-frame rules: building residency and fallen state (`building_layer.gd`), scenery tiers, dressing cells, felled and cleared state (`scenery_layer.gd`, `felled.gd`), the props' fit and tiers (`prop_layer.gd`). It places nothing | The browser exports placement; a live client must derive state anyway | Agent (#3) |
| The browser exports a `props` section through the page (`describeProps`, beside slice 14's `describeUnits`): the map's props fitted once (`props-map.u32`), `PropAppearances.describe()` (candidates, modular, wreck, stand-in), part props per building and gutted shells' prop types. `scenery.felled` carries `biome.trees.felled` | Fallen buildings need the parts and shells; known props need the fit's tables; the map's fit is static | Agent (delegated: the export's home) |
| `props.detail` repeats `presentation.models` beside `units.detail` | Each section stands alone; one source in the browser. Slice 18 may fold them | Agent; open |
| Native settles each frame at once: residents nearest first by record count against the pool, dressing nearest-first within the slots. The browser expands over a few frames and packs its pool; a view past 75% of the pool is a blocker | Plates are mid-shot, where the browser has settled; counts equal on all 24 plates | Agent (spend margin on simplicity) |
| Every material draws as the browser's model surface (`model_surface.gd`): base × vertex colour × colour scale × texture, tint by the material's share, scenery tinted whole; a cutout's coverage is its normal map's alpha, scaled and dithered by pixel | Godot's imported materials ignored vertex colour and tint, and dropped the railings | Agent |
| Map props follow the browser's models layer: chunks in view, each prop culled and tiered alone, cast a tier coarser, nothing out of view casting; replaced props hidden by key | Slice 09's chunk-tier statics drew finer tiers and out-of-view casters | Agent (#3) |
| `model_batches.gd` keeps appearances and chosen instances only; statics, movers and the kit shells' open-tier rule are deleted. Layers cull through `chunk_grid.gd` and write batches only when their plan changed | Every caller now chooses its own instances; one owner of culling cells | Agent (refactor-clean) |
| Felled trees are posed on the CPU (`felled_point`, the port of `felledPoint`, tested against the browser's vectors) and re-baked only when pose or tier changes | At most 14 in the reel; a shader port would need a second surface | Agent |
| The fixture run's manifest carries the world sections but not their tables, and does not list `buildings`, `scenery` or `structures` | Paris's building tables are 3.8 MB; its plate's counts are from another run | Agent |
| The learned ground counts changes to what is cleared (`cleared_version`) | Shrubs and dressing re-stand only then | Agent |
| Performance gate: fails on market-town (p50 22.8 ms against 15.9, p95 35.9 against 18.2); paris passes. CPU fixed from 14.9 to 9.4 ms on records 1000–1299; the rest is props' GPU (about 10 ms), which follows the browser's own drawing rule | Not closed by loosening the bar; recorded with its measured causes and the untried levers in the slice | Agent; open, needs a decision |

### Slice 17: effects

| Decision | Why | Closed by |
| --- | --- | --- |
| The manifest's `effects` carries the tuning as `gameEffects` resolves it and the catalog rows the effects read (hulls, mount muzzles, smoking prop kinds, wreck pieces); units gain the muzzle socket, muzzle nodes and rotor blade facts | The browser resolves, native ports the per-frame derivations, as for the light and the units | Agent (#3) |
| `effectCounts` counts what `EffectFrame` has running at the plate's clock by kind, the instances it built by shape and its lights; started effects only | Exact and load-free; a publication the browser decoded ahead of the drawn tick only adds unstarted effects, so started ones are the same on both sides | Agent (#4) |
| Native reads a record's `simTime` to the nearest double (`core/js_text.gd`) rather than through Godot's JSON reader | Godot's reader is an ulp off on clocks the browser writes; plate 1335's clock sits exactly on a tick and lost 18 tracers. Other JSON numbers stay Godot's (no boundary depends on them) | Territory |
| JavaScript's number text is ported exactly (float32 inputs, shortest round trip, ties to even, exact decimal arithmetic) | The browser seeds dust, smoke sources and tracer chance from keys with positions in them; any other text seeds other particles | Territory |
| The launch tracker moves to `effects/launches.gd` as the one owner; the pose feed reads it over `effect_feed.gd`'s publication | The browser has one `LaunchTracker` for flashes and firing soldiers | Agent (refactor-clean) |
| The effect pass is a compositor raster pass between the haze and the post chain, not Godot transparent geometry | The browser draws its effects after its world (hazed) and before post; a spatial material would be hazed by the depth behind it and blend through Godot's own state | Agent (delegated: drawing technique) |
| Cast lights reach the world's materials through three shader globals (count, wrap, a 48×1 texture) declared in `project.godot`, added to emission with the browser's formula | Every lit material reads them, as the browser's `shade` does; Godot's omni lights have another falloff and BRDF | Agent (#3) |
| Lit smoke's sky light is the baked sky averaged over each hemisphere, cosine-weighted, times the fill; reported as `effects.sky` | Native has no prefiltered environment to read at full roughness; the dust and smoke match the browser's brightness on the plates | Agent; open |
| Sprites nearer than the near plane are clipped, as the browser's reverse-Z projection clips them | Found by the unprimed critique on paris plate 1206 | Agent |
| Not ported: a cook-off's moving wreck and its thrown debris; reported per run as `effects.cookOffs` | They are models drawn through the props' and units' layers, not effects, and no count covers them; native shows the static wreck | Agent; open |
| No `crashes`/`guided` fixture | Neither control publishes them; a WASM stream needs a helicopter shot down, which the time-boxed attempt did not produce. The fixed-cause vectors cover the smoking airframe and rotors | Agent; open |
| Effects cost about 1.3 ms at p50 and 2.9 ms at p95 on market-town (about 1.4 ms CPU); judged relative, since slice 15's market-town cost is still open | The bar is re-checked when slice 15 lands | Agent |
