# Native rendering visual parity

## Status and next pickup

**Status:** pre-discovery draft, created after closing `specs/done/native-rendering`. No implementation slice is accepted yet.

**Next pickup:** run the visual-parity discovery session. Resolve the backend admission question, freeze the browser control viewport, decide whether the native presentation manifest is the shared seam, and approve the first slice. Do not tune individual native shaders before those decisions are recorded here.

**Goal:** make the native display-backed renderer visually equivalent to the browser renderer for the authored menu reel while keeping Rust as the only gameplay, observation, visibility, replay, and digest authority. The browser remains shipped until the final promotion gate passes.

**Global TODO:**

- [ ] Decide the admitted Godot backend and its material/shadow/compute capabilities — slice 01.
- [ ] Freeze a matched 1280×720 browser/native comparison viewport and projection contract — slice 02.
- [ ] Export a browser-owned `native-visual-presentation/v1` manifest instead of making native re-derive presentation policy — slice 03.
- [ ] Port height, relief, roads, field surfaces, forest floor, and terrain material layers — slice 04.
- [ ] Port the browser grass/vegetation field, including density, crop rows, LOD, exclusion, wind, scars, and stable sampling — slice 05.
- [ ] Preserve authored model identity, material roles, pivots, fit, LOD, buildings, roads, and props — slice 06.
- [ ] Port fixture-owned lighting, shadows, color management, sky, haze, fog, and post-processing — slice 07.
- [ ] Port observed dust, smoke, muzzle/tracer, blast, scar, and grounding effects from published causes — slice 08.
- [ ] Compose all layers, instrument performance, and decide promotion — slice 09.

Update this section before ending every implementation pass. Record the current slice, evidence, unresolved decisions, and the next exact pickup.

## What the finished picture must preserve

The browser controls in `specs/done/native-rendering/visualizations/` are the visual authority. The native result must match the same menu-reel workload, map identity, camera timeline, subject tracking, viewport, and visible authored content. Hardware-independent pixel identity is not required; the visual variables and crop masks below are the acceptance surface.

The review order follows the display stack, while implementation order follows dependency risk:

1. **Comparison frame and camera:** viewport, crop, lens, aspect, target elevation, yaw/pitch convention, follow lag, interpolation, clearance, depth convention, and framing.
2. **Sky and horizon:** zenith/horizon colour, cloud coverage and scale, sun disk or glare, aerial perspective, height haze, and horizon stability.
3. **Terrain substrate:** height grid, relief, slope seating, roads, walks, curbs, fields, forest floor, water edges, scars, and world-anchored material variation.
4. **Grass and vegetation:** plot membership, density, crop rows, species mix, patches, drift, dryness, wind, near/far LOD, tree/hedge/understorey placement, and exclusion around roads, forests, water, props, and units.
5. **Buildings, units, and props:** authored template/module identity, pivots, transforms, material roles, alpha/cutout semantics, texture channels, tier selection, articulation and pose, roads, fences, cars, wrecks, and city composition.
6. **Lighting and materials:** fixture-owned sun, ambient/fill, cascaded shadows, contact/AO, roughness/specular/normal/emissive roles, exposure, tone mapping, bloom, and colour management.
7. **Effects and grounding:** dust trails, smoke, muzzle flashes, tracers, blasts, craters, scorch, tracks, trample, contact shadows, selection/range grounding, depth ordering, and fog interaction.
8. **Composition and cost:** occlusion, culling/LOD, dense-map coverage, readable subject scale, display-backed captures, CPU/GPU/transfer/memory telemetry, and the final promotion decision.

A visual slice changes one primary variable. Other visible differences are explicitly out of scope until their slice; whole-frame approval belongs only to slice 09.

## Ownership and boundary contract

| Concept | Owner | Native responsibility |
| --- | --- | --- |
| Rules, fixed ticks, observations, visibility, replay, digests | Rust simulation and publication layout | Decode and draw; never simulate or infer |
| Workload identity, camera stream, viewport and capture hashes | Browser graphics-test capture | Consume the immutable record and report mismatches |
| Map admission, height/relief, surfaces and authored placement | Saved-map exports and their browser resolver | Consume resolved records; never re-parse policy ad hoc |
| Appearance identity, material roles, pivots, LOD tiers | `packages/scene-assets` catalogs and bundles | Import the named source and preserve roles; report fallback |
| Biome, terrain, grass, light and atmosphere parameters | Fixture data evaluated by browser renderer owners | Consume an evaluated presentation manifest |
| Native node/material/shader translation | One Godot adapter and one frame orchestrator | Map manifest data to Godot resources and expose failures |

The preferred seam is an immutable `native-visual-presentation/v1` manifest beside each existing `battle-presentation-capture/v1` file. It carries workload/map hashes, matched viewport and plate crop, full camera poses plus lens/projection fingerprint, resolved terrain/forest/building/prop/grass identities, material roles, evaluated environment values, fog/ground references, and explicit fallback/omission reasons. Large geometry may remain in hashed artifacts, but ordering and ownership stay in the manifest.

Native must not choose a building shell by family/token heuristics, re-read raw map policy, invent visibility, replace a missing asset silently, or add a simulation clock. A parity report must name every fallback and omission with owner, reason, source id, budget, and cut.

## Browser owners to port from

- Camera and projection: `packages/renderer-core/src/camera3d.ts`, `cameraUniform.ts`, `cameraController.ts`, `depthContract.ts`, `apps/battle-lab/src/gameCamera.ts`, and the menu reel in `apps/battle-lab/src/menuReel.ts`.
- Terrain and grass: `packages/battle-renderer/src/terrain/terrainSurface.ts`, `packages/battle-renderer/src/terrain/terrainGrid.ts`, `packages/battle-renderer/src/frame/terrainHeights.ts`, `packages/battle-renderer/src/frame/terrainMaterial.ts`, `packages/battle-renderer/src/terrain/biome.ts`, `packages/battle-renderer/src/terrain/grassField.ts`, and `packages/battle-renderer/src/frame/grassPass.ts`.
- Models and authored content: `packages/scene-assets/src/{loader.ts,schema.ts,templateLibrary.ts,appearanceCatalog.ts}`, `packages/battle-renderer/src/models/{modelLayer.ts,buildingLayer.ts,buildingPlacements.ts,propAppearance.ts,modelTextures.ts,poseDriver.ts}` and scenery modules.
- Light and atmosphere: `packages/battle-renderer/src/light/{sceneLight.ts,physicalEnvironment.ts,aerialParameters.ts,cascadePolicy.ts}`, `packages/battle-renderer/src/world/{environment.ts,sky.ts,shadow.ts,post.ts}` and `packages/battle-renderer/src/frame/{fogMaskPass.ts,fogTerm.ts}`.
- Effects and scars: `packages/battle-renderer/src/effects/{effectFrame.ts,effectPass.ts}` and `packages/battle-renderer/src/frame/{scarTexture.ts,grassPass.ts}`.
- Native consumer to reshape: `native/godot-spike/reel.gd` and `presentation_capture.gd`; lifecycle contracts live in `reel_lifecycle_test.gd`.

## Known evidence and risks

The closed probe proves capture decoding, authored admission, fog replay, named-cut lifecycle and display backing. It does not prove parity. The final report has `comparison_ready=false`, null native GPU/CPU/memory telemetry, sparse/capped vegetation, a flat native substrate, different camera projection, simple lighting, no browser-equivalent effects, and Godot `gl_compatibility` rendering.

The browser controls were recorded at 1280×800 CSS pixels and cropped to 1280×720 plates. A future parity run must record both clients at the same viewport before judging camera, density, or performance. The archived native average FPS is not comparable to the browser report: the browser run was background-throttled and the native report measures only process-frame intervals.

The largest technical risk is backend capability. Forward+ versus GL compatibility must be tested with one representative building, grass, shadow, and sky cut before the team spends time porting shaders that the admitted backend cannot express. Git-LFS pointers, imported material translation, coordinate handedness, depth convention, resource lifetime, culling budgets, and effect-capture coverage are hard risks, not screenshot-tuning details.

Effects require a real publication seam. The current native decoder establishes own units, fog, and ground, but does not expose every effect cause as semantic output. Native must extend the browser/Rust-owned capture layout or explicitly defer an effect; it must never infer dust or smoke from camera motion or map geometry.

## Verification contract

Every visual slice must produce a runnable display-backed browser/native pair and a focused crop or mask for its one variable. Run [`compare-screenshots`](../../.agents/skills/compare-screenshots/SKILL.md) against the archived controls or an approved fresh control, then run [`screenshot-critique`](../../.agents/skills/screenshot-critique/SKILL.md) with an unprimed reviewer as the last visual check. Open shots with [`preview-shots`](../../.agents/skills/preview-shots/SKILL.md) at the non-blocking human checkpoint; if no response arrives, record the evidence-based decision and continue.

Counters never prove pixels. Headless Godot proves lifecycle only. Every pass keeps the Rust digest/replay tests, capture parser tests, native lifecycle tests, and the browser scenes it touches green. The final slice runs the full repository check and verification suite once, with any unrelated baseline/toolchain failure named rather than repinned.

## Next Agent Prompt

You are resuming the native visual-parity draft after the browser/native probe. Start with slice 01. Read this README, `specs/done/native-rendering/README.md`, the archived visualizations, and the browser/native owners listed above. Run the discovery questions before implementation: admit the Godot backend, freeze the viewport, decide the manifest seam, and decide which effect causes the current capture must publish. Do not tune grass, lighting, or materials against the old 1280×800-to-1280×720 crop. After each slice, update this handoff, append choices, run focused contracts and both visual gates, and commit a small pass. Keep the browser shipped and Rust authoritative until slice 09 turns every gate green.
