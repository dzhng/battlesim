# 18b — The presentation API

**Contract:** one small, documented, data-oriented API between the Rust side and every renderer. This file is the reference a renderer author reads. Slices 19, 20 and 20b build only to it, and a change to it is a reviewed spec change. Diagram: [`visualizations/presentation-api.html`](../visualizations/presentation-api.html).

**Gate:** the user reviews this document before anything renderer-facing is built. Naming inside the design is delegated and may change at review. The open questions are at the end.

## The API in one screen

The presentation crate (`crates/presentation`, slice 20) owns one object per viewed side, `Presentation`. It has five calls, the same in the WASM and GDExtension bindings:

| Call | When | Returns |
| --- | --- | --- |
| `load_world(input)` | once per battle | `World`: what doesn't change while the battle runs |
| `push_publication(words, arrived_s)` | each tick's side publication, as it arrives | nothing; errors on a stale or malformed record |
| `clock(wall_s)` | live only, once per drawn frame | the presentation time on the sim clock, seconds |
| `frame(input)` | once per drawn frame, in order | `Frame`: what to draw this frame |
| `observation()` | when the UI redraws | the decoded side observation (slice 19's typed view), for the game UI only |

There are no per-entity calls, no callbacks and no renderer-to-Rust queries. A renderer has two entry points of its own, `load(World)` and `draw(Frame, overlay)`. They replace today's twenty-odd setters.

**`World`** is a set of flat record arrays, valid for the battle's life. It holds the appearance table, the looks table, the light, and the word tables that terrain, grass, scars and fog read.

**`Frame`** is a set of flat record arrays for one drawn frame. It carries only what changed, plus what moves every frame:
- the view and the clock;
- instance batches by appearance, tier and pass, rewritten only when their contents change;
- joint palettes;
- effect instances in draw order, and the cast lights;
- fog eyes and occluders;
- table writes (scars, cleared ground);
- pick boxes and sound emitters, for the UI and audio;
- counts, for the gates.

**The determinism rule.** `frame()` is a pure function of `load_world`'s input, the publications pushed and the sequence of `frame` inputs. It reads no wall clock and no unseeded randomness. Two clients fed the same capture produce byte-identical `Frame`s, and a Frame digest proves it. Presentation reads only the side's publication, so it can't see past fog of war. It never touches the simulation's state, so tuning it never changes a battle digest or a replay.

## Where the line falls

**Rust owns everything per entity:** per unit, soldier, joint, building, chunk, effect, light and eye. Decoding, interpolation, poses, launches, effects, cook-offs, felled and fallen state, static placement, tier and residency choice, culling of batches, light selection, colours by role, pick boxes and counts all live in Rust.

**The renderer owns everything per vertex, texel, pixel and blade,** plus its own resources:
- loading an appearance's meshes and textures by name;
- materials and shaders: skinning by the given palette, wind, a felled tree's bend by its `param`, terrain and grass shading, grass clump generation from the World tables, fog horizon maps from the given eyes, the sky lookup bake, shadow cascade fitting to the given splits, post;
- packing table writes into its own carrier (a WebGPU storage buffer, or a Godot word texture, since Godot can't wrap `R32_UINT`);
- converting sim space to its engine's space, once at the root;
- the overlay pass that draws the UI's meshes.

**A renderer must not** decode a publication, interpolate, choose a tier or a resident, cull an entity, pick or cap lights, seed randomness, read gameplay or presentation tuning from fixtures, or name a unit kind, side, state or order. If a renderer needs a fact of that kind, it goes into a record, as data.

## Conventions

These hold for every record:

| Rule | Value |
| --- | --- |
| Space | Sim space: right-handed, `x` and `y` on the ground, `z` up, origin at the map's origin. Godot converts `(x, y, z) → (x, z, −y)` once, in the native adaptor's root transform; no record is in Godot space. |
| Length | metres |
| Angle | radians. A yaw is counter-clockwise from `+x` (`contract::heading`). |
| Time | seconds on the sim clock (`tick / tick_hz`); one clock, `Frame.clock`. |
| Colour | linear RGB, not premultiplied; alpha is opacity or coverage as the field says. |
| Affine | `[f32; 12]`, a 3×4 row-major model-to-world matrix: three rows of `(basis x, basis y, basis z, origin)`. This is Godot's MultiMesh order and a valid instance vertex attribute in WebGPU. Scale is folded in. |
| Projection | `[f32; 16]` column-major. Reverse-Z with an infinite far plane (`depthContract.ts`). |
| Records | `#[repr(C)]`, a multiple of 16 bytes, fields only 4-byte scalars or arrays of them, no `vec3`. The same bytes are then valid as a WGSL struct, a std430 GLSL struct, a vertex instance attribute and a Godot MultiMesh row. |
| Indices in floats | A field that must sit in a float-only carrier (the Instance tail, read as MultiMesh colour) holds an integral `f32`, exact below 2²⁴. There are no bit casts. |
| Ids | `u32`. A unit id is the publication's id. An index into a World table is that table's row. `NONE = u32::MAX`. |
| Lifetime | In WASM, a view over module memory is valid until the next call into the module, because memory may grow. Upload, then call again. |

## Inputs

**`load_world(input: WorldInput)`**, a rare call, so its input is bytes and JSON:

| Field | Type | What |
| --- | --- | --- |
| `map` | bytes | The saved map, as the sim's map owner admits it (20b) |
| `catalogs` | JSON | The scene-assets appearance catalog and the unit catalog, the same files both clients already load |
| `tuning` | JSON | `fixtures/game.json`'s `presentation` block: effects, pose feel, light, fog look, colours |
| `layout` | JSON | The battle's publication layout (`observation_layout`) |
| `side` | `u32` | The side presented; every publication pushed must be this side's |

**`push_publication(words: &[u32], arrived_s: f64)`** takes the words `Publisher::publish` packs, unchanged; the browser's `Float32Array` is the same bytes. `arrived_s` is the wall time of arrival, read only by `clock`. A capture replay passes 0.

**`frame(input: FrameInput)`**:

| Field | Type | Unit, frame | What |
| --- | --- | --- | --- |
| `time` | `f64` | s, sim clock | The presentation time: `clock(wall_s)` live, the record's `simTime` in a capture |
| `camera.eye` | `[f64; 3]` | m, sim | Eye position |
| `camera.target` | `[f64; 3]` | m, sim | Look-at point |
| `camera.up` | `[f32; 3]` | unit, sim | Up vector |
| `camera.fov_y` | `f32` | rad | Vertical field of view |
| `camera.near` | `f32` | m | Near plane |
| `camera.size` | `[u32; 2]` | px | Drawing size in device pixels |
| `ui.selected` | `&[u32]` | unit ids | Selected own units, for their look |
| `ui.hovered` | `u32` | unit id or `NONE` | Hovered unit |
| `ui.show_orders` | `bool` | — | Space held: ordered destinations draw as ghosts |
| `ui.ghosts` | `&[Ghost]` | — | Ghosts the UI places (purchases, a right-drag's destinations) |

`Ghost` (16 B): `kind: u32` (unit type row), `x, y: f32` (m, sim), `yaw: f32` (rad). Rust fields the ghost's soldiers, poses them and gives them the ghost look. A ghost's validity colour comes from `look`; a purchase the sim refused uses the blocked look.

The camera is an input because the camera controller is UI (input handling and clearance). Rust returns the matrices, so projection has one owner.

## `World`

| Set | Record (bytes) | What |
| --- | --- | --- |
| `header` | `WorldHeader` | Offsets and lengths of every set; `tick_hz: f32`; `extent: [f32; 4]` (min x, min y, max x, max y, m); `side: u32` |
| `strings` | `u8` | UTF-8 arena that names point into |
| `appearances` | `Appearance` (16) | One row per appearance any batch may draw |
| `looks` | `Look` (32) | One row per (side tint, role colour) pair any instance may use |
| `light` | `Light` (one) | The resolved sun, fill, sky, haze, exposure and cascade policy |
| `tables` | `Table` (16) | Named word tables, with their words in `words` |
| `words` | `u32` | The tables' words |

**`Appearance`** (16 B):

| Field | Type | What |
| --- | --- | --- |
| `name` | `u32` | Offset into `strings` of the catalog name the renderer loads |
| `name_len` | `u32` | Bytes |
| `tiers` | `u32` | Authored detail tiers, 0 finest; a card tier, where one exists, is last |
| `skin` | `u32` | Joints a palette for it holds (0: rigid) |

**`Look`** (32 B): `tint: [f32; 4]` is the side tint (linear rgb, and the material share in `a`). `role: [f32; 4]` is the role colour (x-ray own, x-ray selected, enemy, ghost, purchase blocked; linear rgb, alpha; all zero for none). Colours stay in the fixture. Rust turns a unit's side and role into a row, and the renderer only multiplies.

**`Light`:** the values the browser's `LightPresentation` resolves to, with the sun as a unit vector toward the sun in sim space. Its fields are the existing `light` manifest section, unchanged in meaning, and the generated layout names each one. The sky lookup is baked by each renderer from these.

**`Table`** (16 B): `name: u32`, `name_len: u32`, `first: u32` (word), `len: u32` (words). The tables are the ones the shaders already read: terrain `heights`, `nodes`, `plots`, `surfaces`, `surfaceIndex`, `params` and `water`; grass `params`, `props`, `shapes`, `indicesNear` and `indicesFar`; `scars` (its look, normalisation and empty directory); fog `heights` and `foliage`. Their record layouts are the existing `d.struct`s, moved into the generated layout (below). Market-town's tables total about 16 MB, most of it the surface index (6.6 MB) and grass props (7.4 MB).

Static placement is **not** in World. Market-town places well over a million instances (*est.*: the dressing alone is 10.8 M words, about 900,000 rows at 12 floats, and the intact building modules 7.9 M words). Putting them in World would upload all of them to draw at most 63,000. Placement lives in Rust, and the selected instances arrive through Frame batches (open question 3).

## `Frame`

| Set | Record (bytes) | Rate |
| --- | --- | --- |
| `header` | `FrameHeader` | every frame: offsets and lengths of every set, `clock: f64` (s, sim), `tick: u32` (the publication drawn), `index: u32`, `light_wrap: f32`, `occluders_version: u32` |
| `view` | `View` (one) | every frame |
| `batches` | `Batch` (32) | every frame: the list of what is drawn |
| `instances` | `Instance` (64) | only for batches whose contents changed |
| `palettes` | `Joint` (48) | every frame, whole |
| `effects` | `Effect` (64) | every frame, in draw order |
| `lights` | `CastLight` (32) | every frame, at most 24, already chosen |
| `fog_eyes` | `Eye` (48) | every frame, when fog is drawn |
| `fog_occluders` | `Occluder` (32) | when the side's knowledge changes (`header.occluders_version`) |
| `table_writes` | `TableWrite` (16) | on change, with their words in `words` |
| `picks` | `Pick` (48) | every frame; the UI's, not the renderer's |
| `emitters` | `Emitter` (48) | every frame; audio's |
| `counts` | `Counts` (64) | every frame; the gates' |

**`View`:**

| Field | Type | Unit, frame |
| --- | --- | --- |
| `view` | `[f32; 16]` | sim → view, column-major |
| `projection` | `[f32; 16]` | reverse-Z, infinite far |
| `eye` | `[f32; 4]` | m, sim; `w` unused |
| `size` | `[f32; 4]` | px width, height; `1/height`; pixels per metre at 1 m |
| `cascades` | `[f32; 4]` | split distances, m from the eye; the renderer fits its shadow matrices to them |
| `sun_penumbra` | `[f32; 4]` | the per-cascade softness the light policy gives |

**`Batch`** (32 B):

| Field | Type | What |
| --- | --- | --- |
| `key` | `u32` | Stable id for this batch across frames; the renderer keys its GPU buffer by it |
| `appearance` | `u32` | `World.appearances` row |
| `tier` | `u32` | Detail tier |
| `pass` | `u32` | 0 drawn in view, 1 shadow caster |
| `cascades` | `u32` | Casters: bit *n* set if it reaches cascade *n* |
| `version` | `u32` | Changes when the contents change |
| `first` | `u32` | First `Instance` of the new contents, or `NONE` when unchanged since the renderer last saw this `version` |
| `count` | `u32` | Instances drawn |

A batch missing from the list is not drawn this frame; the renderer may keep its buffer for when it returns. Units, ghosts and corpses are batches like any other, and theirs change every frame. Buildings, scenery, dressing, props and stumps change when a chunk changes tier or residency, or the side learns a fall.

**`Instance`** (64 B):

| Field | Type | Unit, frame | What |
| --- | --- | --- | --- |
| `affine` | `[f32; 12]` | m, sim | Model to world, with scale (a fitted prop's per-axis fit) and tilt (a falling airframe) folded in |
| `palette` | `f32` | integral | First `Joint` of its palette; 0 is the identity entry (rigid) |
| `look` | `f32` | integral | `World.looks` row |
| `param` | `f32` | per appearance | The one per-instance scalar a material reads: a felled tree's fall progress (0..1, toward its local `+x`), a card's atlas layer |
| `pad` | `f32` | — | 0 |

**`Joint`** (48 B): `affine: [f32; 12]`, the bind-to-world motion of one joint or articulated node, in the instance's model space. A soldier's palette has its appearance's `skin` joints. A vehicle's palette has one per articulated node; its first entry carries the track scrolls in its origin row (`travel_l`, `travel_r`), as native does today. Rust samples the clips: the clip tables never reach a renderer (open question 1).

**`Effect`** (64 B), four `vec4` whose meaning depends on `shape` (`misc.x`): 0 streak, 1 glow, 2 flipbook, 3 rotor. It is today's `EFFECT_FLOATS` record unchanged. The generated layout names each field per shape:

| Field | Streak | Glow, flipbook | Rotor |
| --- | --- | --- | --- |
| `a` | tail end xyz (m, sim); width m, or −px for a fixed line | centre xyz; radius m | hub xyz; reach m |
| `b` | head end xyz; least width px | spin rad; least px; flipbook layer and frame | axis xyz; sweep |
| `colour` | linear rgb; opacity (0: additive) | linear rgb; opacity (0: additive glow) | blade direction xyz; shade |
| `misc` | shape; texture along at a, at b; boost | shape; —; lit flag; boost | shape; chord over reach; blades; least blade px or cover |

Effects are ordered as they blend, and draw in array order.

**`CastLight`** (32 B): `position: [f32; 4]` (xyz m, sim; `w` = 1/radius² in m⁻², 0 for a light that reaches nothing) and `colour: [f32; 4]` (rgb irradiance at its centre; `w` unused). `header.light_wrap: f32` carries the facing wrap. These are exactly the GPU's `CastLights` rows: Rust has already culled them to the view, ranked them and capped them at `CAST_LIGHTS_MAX`.

**`Eye`** (48 B): `position: [f32; 4]` (m, sim; `w` = range m); `shape: [f32; 4]` (front, side and rear multipliers of range; forward yaw, rad); `key: u32` (stable per unit and eye, so an eye that did not move keeps its horizon map); then 12 B padding. **`Occluder`** (32 B): a known structure's box, centre xyz and yaw, half-extents xyz, as `fogInputs.ts` packs it today.

**`TableWrite`** (16 B): `table: u32` (a `World.tables` row), `offset: u32` (words into the table), `len: u32` (words), `first: u32` (into `words`). The writes cover the scar table's changed tiles (the native word layout, one directory word per 16-cell tile and a 256-word page per marked tile) and foliage cleared by the side's learned ground.

**`Pick`** (48 B): `unit: u32`, `member: u32` (or `NONE` for the whole unit), `flags: u32` (own, identified, corpse, aloft), a pad word, `centre: [f32; 4]` (m, sim; `w` = yaw rad), `half: [f32; 4]` (half-extents m; `w` unused). These are the boxes of what this frame draws, so the UI's hit tests agree with the picture.

**`Emitter`** (48 B): `unit: u32`, `member: u32`, `class: u32` (the vehicle class's row, or soldier), `flags: u32` (enemy, reversing), `position: [f32; 4]` (m, sim), `motion: [f32; 4]` (left and right track travel m, turret yaw rad, unused). This is `soundMotion` as data, since the TS poses it reads are deleted.

**`Counts`** (64 B): soldiers, vehicles, aircraft, corpses, fading, then running effects by kind (tracer, flash, impact, sparks, blast, puff, flame, trail), then lights, then instances dropped. These are the per-plate counts the parity gates compare today. Once both clients run one crate, the gates change meaning: they check that each renderer drew what its `Frame` listed, and Frame digests prove that both clients got the same `Frame`.

## One definition, two bindings

The records are defined once, in `crates/presentation/src/records.rs`, through a `records!` `macro_rules` macro. For each record it expands to:
- a `#[repr(C)]` struct deriving [`bytemuck`](https://crates.io/crates/bytemuck)'s `Pod` and `Zeroable` (1.25, with `derive`);
- a compile-time size assertion;
- a `Layout` constant listing each field's name, offset, scalar type, count and the doc comment that states its unit and frame.

A test (`records_are_current`) renders every layout to two generated files and fails if either differs from what's checked in:
- `packages/battle-renderer/src/records.gen.ts`: TypeGPU `d.struct` schemas (`d.unstruct` for vertex layouts), byte sizes, set names, and a `frameViews(memory, ptr)` / `worldViews(memory, ptr)` pair. Each builds typed-array views over WASM memory from the header.
- `native/godot/shaders/records.gen.glsl`: std430 structs for the compute and raster passes that read storage buffers (effects, lights, eyes).

Nothing is maintained by hand on either side. A field change regenerates both, and a stale file fails the test.

**WASM.** A `#[wasm_bindgen]` `Presentation` class exposes the five calls. `frame()` returns a pointer to its `FrameHeader`, the pattern `Battle.publication_ptr` already uses (`web/src/battle/sim/authority.ts`). The generated `frameViews` reads the header through one `Uint32Array` and builds a `Float32Array` or `Uint32Array` view per set. The renderer passes each view straight to `queue.writeBuffer`, with no intermediate copy and no JS object per entity. No `js-sys` is needed. Presentation runs on the main thread, where the frame is drawn; the sim stays in its worker.

**GDExtension.** The native core is Rust, so it uses the structs directly; there is nothing to bind. Its Godot adaptor writes:
- each changed batch through `RenderingServer::multimesh_set_buffer(rid, &PackedFloat32Array::from(bytemuck::cast_slice(run)))`, which godot-rust 0.5.5 has. The Instance layout is Godot's 3D MultiMesh row with colour, so this is one copy into Godot's buffer;
- palettes, effects and lights through `RenderingDevice::buffer_update`, or `texture_update` for the palette texture;
- table writes into the word textures.

GDScript never sees a Frame.

Considered and not used:
- [`ts-rs`](https://crates.io/crates/ts-rs) (12.0) generates TS interfaces for serde objects, not memory layouts, so it would describe the wrong thing.
- `zerocopy` does what `bytemuck` does with a larger surface.
- A proc-macro crate adds a crate where a `macro_rules` macro suffices.

## The three ways a frame is drawn

- **The browser's live game.** The worker posts each publication. The main thread calls `push_publication(words, now)`. On each animation frame it calls `frame({ time: clock(now), camera, ui })`, and then `renderer.draw(frameViews(...), overlay)`. The UI reads `observation()` once per new publication, plus the `picks` of the frame it drew.
- **The native capture replay.** The core calls `load_world` from the capture's map and side. For each frame record in order, it pushes every sample up to the record's tick, then calls `frame({ time: record.simTime, camera: record's eye, target, up and the manifest's lens, ui: empty })`. It draws plates where the record names one. Poses and effects carry state from frame to frame, so every record is framed, including those a plate run skips, as today. The manifest keeps identity, viewport, lens, frame records and counts. Placement, units, props and effects tuning no longer travel through it (20b).
- **The browser's graphics-test capture** is the live path with the capture writer attached. Its control plates and the native plates come from the same `Frame` bytes.

**Labs.**
- **Labs that run a battle** draw through the same path as the game: `fog-look`, `fog`, `weapons`, `cookOff`, `airCrash`, the air labs and the others on `useBattleSession`.
- **Records are plain data, so a lab may edit them** before handing them to the renderer. There is no third path and no lab-only call.
  - `fog-look` edits the fog look record it copied from `World` to tune live, and filters `Frame.fog_eyes` for its recon-only and scene-placed eyes.
  - The grass toggle is a renderer drawing option.
- **Labs with no battle** (`foundation`, `geometry`, the workbench sheet) build `World` and `Frame` records themselves with the generated schemas, for example a batch of one appearance. `setInstances`' proxy boxes become a proxy appearance's batch.
- **`ballistics`** builds its own `EffectFrame` from WASM ballistics runs. It has no battle, so it has no publication. It draws its trajectories as overlay lines instead (open question 6).

## What each call site becomes

**Browser** (`packages/battle-renderer`; every setter call is in `apps/battle-lab/src/LabViewport.tsx`):

| Today | Becomes | Deleted |
| --- | --- | --- |
| `setModels`, `setCorpses` from `useBattleSession.frame` (`interpolate.ts`, `poseFeed.ts`, `poseDriver.ts`, `modelInstances.ts`, `drawnModels.ts`, `cookOffModels`, `debrisModel`, `pushGhostModels`, `corpseInstances`) | `Frame.batches`, `instances`, `palettes`; ghosts via `ui` | those builders, `poseKernel.ts` and its WGSL, the clip-table upload, `cookOffs.ts`, `sideInstances`' pick boxes |
| `setEffects`, `setClock` (`effectFeed.ts`, `effectFrame.ts`, `launches.ts`, `cookOff.ts`, `castLights.ts`' selection) | `Frame.effects`, `lights`, `header.clock` | all of them, except `effectPass.ts` and the light's GPU struct |
| `setStructures`, `setBuildings`, `setFelled` (`sideStructures`, `knownFallen`, `destroyedBuildings.ts`, `buildingPlacements.ts`, `buildingReferences.ts`, `placementPool.ts`, `propAppearance.ts`, `felled.ts` CPU side) | `Frame.batches`; a felled tree's `param` | all of those, and `buildingLayer.ts`' expansion |
| `setWorld` (`worldMesh.ts`; `scenery/placement.ts`, `dressing.ts`, `lod.ts`; `templateLibrary.ts`; the terrain and grass packers) | `World.tables`, scenery through `Frame.batches` | the resolvers and packers (20b) |
| `setGround` (`ground.ts`, `groundPage.ts`) | `Frame.table_writes` | the client ground view and the scar page cache; `scarTexture.ts` keeps only the upload |
| `setFog` (`fogEyes`, `knownOccluders`, foliage less cleared) | `Frame.fog_eyes`, `fog_occluders`, `table_writes` | `fogInputs.ts`' builders |
| `render(view, camera)` | `frame`'s camera in, `Frame.view` out | `frameCamera.ts`' matrix build |
| `setAppearances` | `World.appearances` names; asset loading stays renderer-side | — |
| `setFogStyle`, light | `World` records | — |
| `setOverlay`, `setPointerMarks` | `draw`'s `overlay`, UI meshes, outside the API | — |
| `setInstances` | a proxy appearance's batch | `frame/geometry.ts` |
| `ViewportFrame.picks`, `counts`, `effectCounts`; `soundMotion` | `Frame.picks`, `counts`, `emitters` | `modelCounts`, `EffectFrame.counts`, the pose read in `soundFeed.ts` |
| `observation.ts`, `client.ts`' decode | `push_publication`; `observation()` for the UI | the decoder (19) |

**Native** (`native/godot`):

| Today | Becomes | Deleted |
| --- | --- | --- |
| `capture/observation.gd`, `learned_ground.gd` | `push_publication` | both (19) |
| `units/`: `tick_interpolator`, `pose_feed`, `pose_driver`, `unit_poses`, `articulation`, `pose_math`, `pose_kernel` and its compute; `unit_layer`'s posing | `batches`, `instances`, `palettes` | all but the palette upload; the 768 KiB vehicle palette rewritten whole each frame goes too |
| `effects/` except `effect_pass.gd` | `effects`, `lights` | `effect_feed`, `launches`, `cook_offs`, `effect_frame`, `drawn_models`, `cast_lights`' selection |
| `world/`: `building_layer`, `scenery_layer`, `prop_layer`, `felled.gd`, `chunk_grid`, `detail_view`, `static_world`; `selection.rs` and `felled.rs` (slice 15's Rust) | `batches`, `instances` | all; `model_batches.gd` becomes the adaptor's batch writer |
| Manifest sections `units`, `props`, `buildings`, `scenery`, `effects` and their artifacts | `load_world` | the sections and the browser's exporters (20b) |
| `terrain_layer`, `grass_layer`, `scar_texture` | `World.tables`, `table_writes` | the scar packing; the uploads and meshes stay |
| `environment/light.gd` derivations | `World.light`, `View.cascades` | the derivations (21); the adaptor stays |
| `fog_layer.gd` (debug cells; no reel draws fog) | `fog_eyes` when native draws fog | deleted |
| `reel/frame_stepper.gd`, `camera_rig.gd` | the replay driver above | moved into the core (21) |

## Sizes on market-town

Measured from control `2026-10-11T15-26-04-627Z` (18 plates); anything marked *est.* is computed from strides.

**Into Rust, per tick (30 Hz).** A delta publication is a median 371 words (1.5 KB) and 843 at p95 (3.4 KB). A fresh record is 37,719 words (151 KB), of which 17,579 are fog.

**Today, per drawn frame:**
- **Browser:**
  - model records: 64 B a model, about 95 models, so 6 KB;
  - pose controls: 64 B a soldier, 4 KB;
  - palette staging: 3.4 KB a soldier, including the skinned slots the GPU kernel then overwrites. That is about 230 KB for 65 soldiers and 3 corpses (*est.*);
  - effects: 246–700 instances at 64 B, so 16–45 KB;
  - lights: at most 784 B;
  - fog eyes: 48 B each;
  - scar pages: the changed ones.
  - In total, about 300 KB.
- **Native:**
  - the effect buffer: the same 16–45 KB;
  - the vehicle palette: 768 KiB, rewritten whole whenever any vehicle is drawn;
  - pose controls: about 1 KB;
  - the scar table: about 1 MB, rewritten whole when a tile changes;
  - MultiMesh buffers whenever a layer re-plans.

**`Frame`, per drawn frame:**

| Set | Market-town | Notes |
| --- | --- | --- |
| view, header, counts | < 1 KB | |
| batches | *est.* a few hundred × 32 B, under 16 KB | native's MultiMeshes today: one per appearance, tier and pass (slice 09's record 1100: 160 view surfaces) |
| unit instances | about 95 × 64 B = 6 KB | every frame |
| static instances | 0 in a still frame; at most 63,054 × 64 B = 4.0 MB when everything re-tiers at once (a cut) | drawn statics per plate run 30,057–63,054, mostly dressing; the typical per-frame change is measured in slice 18 |
| palettes | 65 soldiers + 3 corpses at 53 joints × 48 B = 173 KB, plus vehicles (16, *est.* 23 KB): about 200 KB | at the reel's end, 8 soldiers + 69 corpses = 196 KB |
| effects | 16–45 KB | 246–700 instances on the plates |
| lights | ≤ 784 B | at most 13 on the plates |
| fog eyes and occluders | a few KB | the reel draws no fog |
| table writes | 1 KB a changed scar tile | about 450 tiles over a whole battle |
| picks, emitters | about 95 × 48 B × 2 = 9 KB | |

**About 230 KB a frame with Rust posing, and about 60 KB if poses stayed on the GPU, plus static batch changes.** That is less than either client uploads today, and at 120 fps it is 28 MB/s, which is nothing on unified memory. The cost to watch is not bytes but Rust's CPU time per frame on the browser's main thread. Slices 19 and 20 measure it, as their research steps already require.

## Open questions for review

1. **Poses on the CPU in Rust, or kept on the GPU.**
   - *Recommended: Rust writes joint palettes.* It deletes both pose kernels (`poseKernel.ts`, the Godot compute) and the clip tables' GPU path, and renderers stop knowing about clips. The cost is about 3,600 joint samples a frame, which is microseconds in Rust, and about 200 KB of upload, about what the browser already uploads and throws away.
   - The alternative is that `Frame` carries the 64 B pose controls and `World` carries the clip tables. That is smaller, but leaves a sampling kernel in each renderer.
2. **The browser's plates must stay byte-identical** (choices.md, phase 4). Moving math from shaders into Rust changes float order: a yaw becomes an affine, and a GPU pose becomes a CPU palette. Plates may then move by a level or two.
   - *Recommended:* hold Frame digests, counts and vectors exactly, and judge plates within the existing noise floor, once, in slice 23.
   - Otherwise, records must reproduce today's shader inputs exactly, and question 1 must go the GPU way.
3. **Static placement through Frame batches, not World lists.** The proposal put instance lists by appearance and tier in `World`.
   - *Recommended:* batches written on change. This is what native does today and what Godot's MultiMesh needs.
   - World lists would upload over a million instances to draw at most 63,000, or need an index indirection that Godot's MultiMesh doesn't have.
4. **Overlays stay UI** (order lines, contact glyphs, markers, the range ruler): meshes the browser's UI builds from `observation()` and hands to `draw`. **Decided by the user (2026-10-11): yes.** An order line runs from a unit's position to its members' destinations, which `observation()` already carries (`memberOrders`). Rust describing the line would only repeat that data. Ghosts stay in Rust, because they need the fielding and posing every unit gets.
   - *Recommended: yes, for phase 4.* Native has no UI, and moving overlay geometry to Rust is a UI project of its own.
5. **Fog's look is in `Frame`, not the UI surface.** It is drawn in the world, and a live native client needs it.
   - *Recommended: yes.* The UI keeps the contact glyphs.
6. **`ballistics` loses its effect drawing** and shows trajectories as overlay lines. It is a developer arena with no battle to publish.
   - *Recommended: yes.* The alternative, a lab-only `note_effects` call, is the one exception the API would carry.
7. **The browser's presentation WASM is its own small module** (`presentation-wasm`). It links the reader and the presentation crate, not the rules, so the main thread doesn't load the simulation.
   - *Recommended: yes.* Slice 19 then places the reader where both the sim and this module can link it, beside the publication layout.
8. **Audio reads `Frame.emitters`.**
   - *Recommended:* the record above, since the TS poses `soundMotion` reads are deleted in slice 20.
