# Renderer cleanup: the map

The explore-unknowns walk of 2026-10-30, the brief every slice inherits. The
[README](README.md) and its slices own the plan; this page owns why each
decision was made. Where they differ, the slices win.
Close it into the README's rationale when the spec closes.

## Known knowns

The debt is what the pre-merge review of the shared Rust renderer (PR #16,
merged to `staging` at `73bd05e51`) deliberately left. Every item was checked
against `staging` before planning.

| # | Item | Ground truth |
|---|---|---|
| 1 | WGSL modules don't own their concepts | `crates/renderer-core/build.rs` (97 lines) concatenates `// @block <id>` blocks; `src/shaders/pipeline_sources.txt` (69 lines, one pipeline each) lists `shader\|block,block[from=to;…]`; aliases are whole-token substitutions (`build.rs:3-28`). 22 modules, 5,992 lines, 534 blocks. 146 blocks carry position suffixes (`_alternate` 66, `_primary` 35, `_extended` 22, `_masked` 10, `_layered` 7, `_uniform` 3, others 3); about 85 are binding declarations, of which about 50 (in 17 sets) differ only in `@group`; the rest differ by binding number, access mode or a mangled name. 222 alias pairs mostly undo compiler mangling (`params=params_1` 44 times); `item_1` (7×) and `_arg_0` (166×) remain. `fogUnpack`/`fogUnpackBuild` (fog_sight.wgsl:251-259) and `fogSample`/`grassSample` (fog_sight.wgsl:225, grass.wgsl:90) are byte-identical. The `{color, fog}` fragment output struct is defined 9 times in fog_sight.wgsl. `Camera`/`Wind` live in fog_sight.wgsl; `TerrainParams`/`WaterLook` in scenery.wgsl; `RoadLook` in effects.wgsl; `ScarParams` in grass.wgsl; `GrassParams` in fog_masks.wgsl; ground functions in models_material.wgsl:85-160. The shadow cascade blend is unrolled 4× (shadow.wgsl:25-80). `shaders/README.md` already states the one-owner rule. |
| 2 | Hand-packed GPU layouts aren't reflected | `LAYOUTS` is built by the `records!` macro (`crates/presentation/src/records/layout.rs:52-95`, records in `records.rs`) and checked against WGSL by `crates/renderer-core/tests/pipeline_abi.rs:51`; every built-in shader is naga-validated at `:126`. Outside it: `TerrainParams` (`static_world/ground/tables.rs`, offsets pinned in `ground/tests.rs:384-397`), `GrassParams` (`ground/grass.rs`), `ScarParams` (`scars.rs:14` `WORDS = 52`, flag word 3), `WaveRun` (`water.rs:119`), the `Environment` prefix (`environment.rs:17` `SUN_AT = 80`; `pipeline_abi.rs:99-113` checks only relative offsets), plus `GroundPaintStyle`/`Paint`, `FogLayer`, `ShoreTuning`, `OverlayGlow`, the `CastLights` header. Literal lengths: `array<WaveSplat, 1536>` (= `STEP_SPLATS` 256, owned by `renderer-core/src/water.rs:112`, × `WAVE.max_steps` 6, a Rust const at `presentation/src/wave.rs:31`), `array<WaterLook, 8>` (`WATER_LOOKS_MAX`), `array<RoadLook, 4>` (`AREA_KINDS.len()`). `536870911u` appears 7×: once the scar `KEY_MASK`, six times the ground field's `RECORD_MASK`. `SKY_LUT_WIDTH` duplicates `pipeline_registry` sizes; `RAYS_BYTES` duplicates a test literal. `shader_constants` reaches WGSL as a runtime-prepended `const` prelude (`pipelines.rs:414-424`). |
| 3 | Scenery tier mesh has two owners | Rust `tier_mesh`/`STATE`/`FOLIAGE` (`renderer-core/src/scenery.rs:30-64`) vs TS `tierMesh`/`SCENERY_STATE`/`FOLIAGE_MATERIAL` (`packages/battle-renderer/src/scenery/appearance.ts`). Identical output. Native already builds tier 0 in Rust (`native/client/src/capture.rs:312-319`); only `web/src/battle/sim/presentation.ts:537` uses the TS copy, for placement (tree scale, felling, stumps). |
| 4 | GPU frame timing has two owners | `native/client/src/bench.rs` `FrameTimer` (lines 44-179) vs `renderer-core/src/timing.rs` (214 lines). Same marker pass and tick check. The bench resolves after the frame completes (Metal stale end timestamps), so it needs per-slot query sets and blocks; it measures every frame against the caller's frame index. Core never blocks, drops frames when busy, keeps a 240-sample window. Core is used only by the browser; bench only by `--bench`. |
| 5 | World geometry built twice | Native: `WorldGeometry::from_triangles` (`renderer-core/src/runtime/world.rs:26-40`). Browser: `web/src/battle/render/sceneGeometry.ts` + `packages/battle-renderer/src/worldMesh.ts:230-252` over the same wasm helpers, then `set_world_geometry`. Browser-only inputs: traversal tint and prop boxes, labs' prebuilt terrain meshes, the geometry lab's backdrop override. The slot is `skirt` in Rust, `props` in TS, `skirt_and_debug_props` in stats. |
| 6 | renderer-core's tests live in native | `native/client/tests/{high_air,impostors,xray,probes,marks,fog_views,frame_views,scar_visibility,model_geometry,pipelines,water_sampling}.rs`: 3k lines, 19 tests, all renderer-core behaviour, through shims in `native/client/src/{lib,pipelines,bundle}.rs` and the device helper `gpu.rs`. `cargo test -p renderer-core` has 20 tests today. renderer-core's `wgpu` has no native backends. |
| 7 | Track nodes decided twice | `renderer-core/src/models/admission.rs:126-130` (name only) vs `presentation/src/appearance.rs:357-360` (name and `link_pitch_m > 0`) vs TS `packages/scene-assets/src/articulation.ts:256-261`. No disagreement today: the bake rejects pitchless tracks (`validate.ts:925-937`). |
| 8 | Generated TS records | `crates/presentation/src/records/typescript.rs` writes `packages/battle-renderer/src/records.gen.ts` (114 exports): 21 read by production code, 11 only by tests or scenes, 82 by nothing. |
| 9 | Wave splat `kind` unread | Set in `presentation/src/effects/frame/wake.rs:117`; `water.wgsl` splats every splat as a ring (:173-179). Fast movers in deep water already drop a trail of V-wake splats along their path (`wake.rs:36-50`); slow movers pulse rings in place. The water spec promised V-wakes (`specs/done/water-and-wetlands/decisions.md` decision 13). |
| 10 | Tests and scenes pin data or shape | `web/tests/proxyModels.test.ts:42-52` copies the gamma formula; `web/scenes/_rulerTour.mjs:40-43,182-184` repeats the first-mount rule and pins two ranges; `web/scenes/_modelGhost.mjs:44-48` pins `basketball_court` and imports ~12 internals; `web/tests/modelWorkbenchSource.test.tsx` mocks six modules; `_appJourney.mjs:29-74` and `_terrainQuery.mjs:4-48` carry near-identical GPU allocation observers and re-wrap `requestDevice` that `_leaks.mjs` already wraps. |

**Proof today.** Native plates come from a browser parity run
(`bun run --cwd web scene -- native-parity-capture`, under the GPU lock) and
`native-client --run <dir> --scene <map> --out <dir>`. Code that is unchanged
reproduces plates byte for byte (`specs/done/one-renderer/research.md:44`). No
committed tool compares two builds.

## Known unknowns: the decision ledger

| # | Question | Decision | Closed by |
|---|---|---|---|
| Q1 | Scope and order | All ten items: layouts, then shader modules, then the two-owner fixes, then tests and brittle tests, then V-wakes last. | User |
| Q2 | Proof | A scratch script per slice (in `throwaway/`): native renders the same parity runs with the base and the changed build, then compares plates byte for byte. No committed tool. | User |
| Q2b | Plate coverage | The menu reel (market-town, paris-corner), the menu reel with `PARITY_FOG=1`, and the water reel (wetlands), plus renderer-core's GPU pixel tests (high air, impostors, x-ray, probes, fog views, scars) and the browser scenes the slice touches. No new map. | Territory: `fixtures/captures/{menu,water}.json`; a paris-corner manifest's layers |
| Q3 | Item 9 | Last slice. First look at the browser water scene: if a V already reads behind fast movers, delete `kind`; otherwise the shader shapes V splats from it and the user signs off the shots. | User (A), revised after reading `wake.rs` |
| Q4 | Windows cutout rewrite (`pipelines.rs:392`) | The discard variant becomes its own WGSL block; the text replace goes; pipeline_abi validates both variants everywhere. The variant is chosen with `cfg!(windows)` in compiled code (`pipeline_registry/models.rs`), not in `build.rs`, which runs on the build host (correction from the planning drafts). | Agent, at the user's request: the only option a rename can't silently break |
| Q5 | Implementer | Codex (gpt-6.1-sol, high) implements each slice; Claude orchestrates, reviews and integrates. | User |
| Q6 | Backward compatibility or migrations | Neither: hard cutovers inside the repo. Nothing outside the repo reads these layouts or generated records. | Agent on the user's behalf (internals) |

**Delegation.** The user opted in. Preferences: scope, what the player sees and
sign-offs are the user's; renderer internals (module layout, slice boundaries,
naming, implementation choices) are answered by the agent, labelled, and open
to override.

## Unknown knowns

- **Who inherits this:** future agents building renderer features (water
  follow-up, shared GPU assets, distant images). Done means they find each
  concept in one place and can't break a GPU layout silently.
- **Definition of done** (accepted by the user with no strikes):
  1. Every shared WGSL struct and function is defined once, in the module named
     for its concept; `shaders/README.md` maps concept to module.
  2. No compiler-mangled names or position suffixes; aliases only where two
     callers genuinely bind different names.
  3. Every buffer layout a shader reads is checked by a test; a Rust size or
     length changed without the shader fails one (raising `WAVE.max_steps`).
     Once lengths are exported the shader follows by construction; the test
     that fails first is written before the export (slice 04).
  4. Tier mesh, frame timer, world geometry and track side each have one owner.
  5. `cargo test -p renderer-core` runs the renderer's own GPU tests; no shims
     in native.
  6. The generated TS records hold only what something reads.
  7. Slices before V-wakes leave every plate byte-identical; V-wakes ship only
     with the user's sign-off.
  8. Net line count goes down; reported, not a target.
- **Nothing in flight collides:** the only open PR re-exports vehicle models;
  one stale branch touches one renderer file.

## Unknown unknowns

Every decision here was answered by the agent on the user's behalf.

1. **A "pure" move can change pixels.** Collapsing near-copies, moving
   constants into the prelude, or looping cascades can change float rounding
   (`shaders/README.md:22`). *Decision:* any byte difference fails the slice
   until traced; put the original expression back unless the change is a
   deliberate maths change, which is then a named decision with diff stats.
2. **The cascade blocks differ:** cascade 0 weights by depth differently
   (shadow.wgsl:36), cascade 3 has no margin (:73). *Decision:* the loop keeps
   first and last explicit and stays byte-identical.
3. **`LAYOUTS` and the TS generator share one record list**
   (`records/typescript.rs:221`). *Decision:* the layouts slice splits them:
   `LAYOUTS` reflects every GPU record; the TS generator takes an explicit list
   of what the page reads. Item 8 moves into the layouts slice.
4. **`536870911u` is two concepts.** *Decision:* export `KEY_MASK` and
   `RECORD_MASK` separately.
5. **Windows rewrite vs renames.** *Decision:* the Windows block lands before
   any WGSL rename.
6. **The browser's GPU times may be stale on Metal** (the core timer resolves
   in-frame, `timing.rs:145`). *Decision:* the core timer gains the bench's
   resolve-after-completion mode; the timing slice checks browser numbers
   against a known workload and switches the browser if they are stale.
7. **Scenery bundles arrive after placement** (`scene.ts:461`,
   `renderer-browser/src/appearance.rs:88`). *Decision:* add scenery kinds to
   source selection as grass does (`appearance.rs:95`), then presentation
   builds tier 0 from the bundles.
8. **Browser-only world inputs.** *Decision:* one Rust constructor takes
   triangles, an optional tint, extra faces and a water switch; labs with a
   prebuilt mesh keep a direct entry; the slot is renamed `skirt_and_props`.
9. **Third track-side copy.** *Decision:* presentation exposes the side and
   renderer-core reads it; the TS bake validator stays (it rejects files before
   Rust sees them); any live TS runtime caller of `articulationRig` that reads
   track sides reads the same exported fact. (Only `articulation.ts:259` and
   `validate.ts:927` match the track names in TS.)
10. **Moved tests need native backends.** *Decision:* a dev-dependency on
    `wgpu` with native backends plus `pollster`, one shared test-device helper;
    `model_geometry` opens the runtime as `tests/grass_assets.rs:18` does.
11. **V-wakes can't be judged on native plates** (native's wave field runs
    fewer steps, `specs/water-native/README.md`). *Decision:* judge them on the
    browser `water` scene, before and after.

**Sharp edges.**
- Build output filled the disk once (73 GB). Reuse one base build per slice;
  children fetch only `assets/runtime` LFS; delete a child's build output after
  integration.
- `bun run test:rust` runs GPU tests without the GPU lock (pre-existing).
- Nothing checks for unused WGSL blocks.
- Stale docs to fix in their slices: `ground/tables.rs:22` ("eight vec4s", ten
  in fact); `specs/done/water-and-wetlands/README.md:559` ("native draws no
  wakes").

## Corrections from the planning drafts

Three independent drafts (fewest slices, risk first, seam quality) found
these; the slices carry them:

- `WaveRun` is 32 bytes in Rust and 24 in WGSL, and has an `i32` the record
  macro can't describe; the macro is private to presentation (slice 02, 04).
- `Sun` is flattened in WGSL, so the `Environment` prefix needs `Sun` nested
  (slice 04).
- The TS generator shares the constant table too, and `SCAR_KEY_MASK` is already
  exported by hand at `records/typescript.rs:86` (slice 02).
- `BUILTIN_PIPELINE_COUNT` counts composition lines and a native test pins it
  to the pipeline count; the Windows variant breaks that (slice 01).
- `build.rs` silently overwrites a duplicate block id (`build.rs:51-60`)
  (slice 05).
- WGSL counts against the 8,000-token file budget, so the ground code splits
  three ways (slice 06).
- Native's `bundle.rs` `Library` is production code; only its re-export goes
  (slice 13).
- Asset replacement already rebuilds scenery tiers in Rust
  (`runtime/assets.rs:48`); only the browser's initial placement uses the TS
  copy (slice 09).
- `_leaks.mjs:5` excludes buffers and textures today; the observer moves there
  (slice 14).
