# Renderer cleanup (placeholder)

**Status:** placeholder. Nothing is sliced or implemented yet.
**Updated:** 2026-10-30.

## Next Agent Prompt

You are picking up the debt the pre-merge review of the shared Rust renderer
found and deliberately left for later. The renderer is `crates/renderer-core`,
shared by native and the browser (`crates/renderer-browser`); its closed spec is
[`specs/done/one-renderer`](../done/one-renderer/README.md). Its bugs, dead code
and stale docs were fixed before PR #16 merged. What's below is structural:
- concepts with two owners;
- GPU layouts no test reflects;
- WGSL modules that don't own their concepts.

None changes what the player sees. Walk the list with the user and agree an
order. Then run `write-spec` and turn this page into a sliced plan. Prove each
item with byte-identical native plates and the browser scenes.

- [ ] Give each WGSL concept one module.
- [ ] Reflect every hand-packed GPU layout.
- [ ] Give the scenery tier mesh one owner.
- [ ] Give GPU frame timing one owner.
- [ ] Build world geometry in one place.
- [ ] Move renderer-core's tests into its own crate.
- [ ] Decide each track node once.
- [ ] Trim the generated TypeScript records to what the page reads.
- [ ] Draw V-wakes, or drop the wave splat's kind.
- [ ] Stop tests and scenes pinning data or implementation shape.

## The items

1. **WGSL modules don't own their concepts**
   (`crates/renderer-core/src/shaders/modules/`). Shared structs and functions
   sit in whichever module first needed them. For example, `Camera` and `Wind`
   live in `fog_sight.wgsl`, and `TerrainParams` and `WaterLook` in
   `scenery.wgsl`.

   Binding variants carry meaningless suffixes (`_primary`, `_alternate`, …),
   and compiler-mangled names remain (`params_1`, `item_1`), with aliases that
   only undo them. There are also hand-copied bodies:
   - `fogUnpack` and `fogUnpackBuild` are byte-identical;
   - `fogSample` and `grassSample` are identical;
   - the fog output struct is defined eight times;
   - the shadow cascade blend is unrolled four times instead of looping over
     `SHADOW_CASCADES`.

   Move each struct and function to its concept's module, name the variants by
   what they are, and collapse the copies. `shaders/README.md` already states
   the rule.
2. **Hand-packed GPU layouts aren't reflected.** These are outside `LAYOUTS`:
   - `TerrainParams`, `ScarParams`, `GrassParams`, `WaveRun`, and the
     `Environment` prefix.

   These layout facts are duplicated as literals:
   - `scars.rs` `WORDS = 52` and its flag word;
   - `environment.rs` `SUN_AT = 80`;
   - `array<WaveSplat, 1536>` (that's `STEP_SPLATS × max_steps`);
   - `array<WaterLook, 8>` and `array<RoadLook, 4>`;
   - `536870911u` (`KEY_MASK`), seven times;
   - `SKY_LUT_WIDTH` and `RAYS_BYTES`.

   Raising `WAVE.max_steps` would read splats out of range with no test failing.
   Export the lengths and masks through `presentation::shader_constants`, add
   the records to `LAYOUTS`, and derive the offsets.
3. **The scenery tier mesh has two owners.** Rust `tier_mesh`, `STATE` and
   `FOLIAGE` (`crates/renderer-core/src/scenery.rs`) are copied in TypeScript
   as `tierMesh`, `SCENERY_STATE` and `FOLIAGE_MATERIAL`
   (`packages/battle-renderer/src/scenery/appearance.ts`).
   `web/src/battle/sim/presentation.ts` places scenery from the TypeScript copy.
   Have presentation derive the meshes from the bundles it already receives,
   then delete the TypeScript copy.
4. **GPU frame timing has two owners.** `native/client/src/bench.rs`
   (`FrameTimer`) repeats `crates/renderer-core/src/timing.rs`; it differs only
   in resolving after the frame completes, a Metal workaround. Give the core
   timer that mode and delete the bench copy.
5. **World geometry is built twice.** Native builds terrain, skirt, backdrop
   and water in Rust (`WorldGeometry::from_triangles`). The browser rebuilds
   them in TypeScript from the same wasm helpers
   (`web/src/battle/render/sceneGeometry.ts`,
   `packages/battle-renderer/src/worldMesh.ts`). Also, the "skirt" slot
   actually carries skirt plus prop faces. Expose one Rust constructor both
   shells call, and name the slot for what it holds.
6. **renderer-core's tests live in the native client.** These test
   `renderer_core` types directly, behind re-export shims:
   - `native/client/tests/{high_air,impostors,xray,probes,marks,fog_views,frame_views,scar_visibility,model_geometry}.rs`;
   - the shims are `native/client/src/lib.rs`, `pipelines.rs` and `bundle.rs`.

   So `cargo test -p renderer-core` skips most of its own coverage. Move them,
   with a shared test-device helper, and delete the shims.
7. **Track nodes are decided twice.** `crates/renderer-core/src/models/admission.rs`
   and `crates/presentation/src/appearance.rs` both match `track_L`/`track_R`,
   with different conditions; presentation also requires `link_pitch_m > 0`.
   Have presentation expose each node's track side, and have admission read it.
8. **`records.gen.ts` publishes the old renderer's schema.** Most of
   `crates/presentation/src/records/typescript.rs`'s exports have no TypeScript
   reader now. Generate only what the page reads.
9. **The wave splat's `kind` is never read.** `water.wgsl` uploads it but draws
   every splat as a ring. Shape V-wakes from it, or drop the field.
10. **Tests and scenes pin data or implementation shape:**
    - `web/tests/proxyModels.test.ts` copies the gamma formula;
    - `web/scenes/_rulerTour.mjs` picks marks by mount index;
    - `web/scenes/_modelGhost.mjs` pins a catalog prop;
    - `web/tests/modelWorkbenchSource.test.tsx` mocks six internal modules;
    - `_appJourney.mjs` and `_terrainQuery.mjs` re-implement the GPU
      allocation observer `_leaks.mjs` owns.

    Assert behaviour instead (`write-tests`).

## Open questions for the walk

- Which of these is worth doing before more renderer features land on top?
  The WGSL module ownership and layout reflection are the riskiest to leave.
