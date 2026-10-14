# 25 — Native draws with wgpu (phase 5, planned)

**Why** (user, 2026-10-11): after slice 15's Rust walk, market-town's remaining gap is on the GPU.
- Native draws the browser's exact instances, but Godot's Forward+ costs more per primitive: a separate depth prepass, per-instance material and shadow work at 4× MSAA, and about 8.6 ms of fixed setup.
- The browser draws through lean passes written for this game.
- Native's shaders are hand ports of the browser's WGSL, about 2,000 lines.

**Contract:** native draws with Rust's `wgpu`, running the browser's own WGSL shaders and pass structure over the 18b API's `World` and `Frame`. Godot leaves native.
- The shaders exist once, used by both clients.
- Native's frame costs no more per primitive than the browser's.

**Seam:**
- One Rust native client: a window (`winit`), `wgpu`, and the presentation crate (20) behind 18b's calls.
- The browser's TypeGPU passes emit WGSL; native loads the same WGSL text. The research step decides how: exported at build time, or shared source.
- Appearances (glTF) load in Rust (`gltf` crate); audio is decided in research.
- **Godot's native project is deleted when native's plates and counts match:** `native/godot`, its shaders and GDScript, the Godot-specific parts of the native core, and the parity harness's Godot runner.

**Reference engine** (user, 2026-10-11): [Bevy](https://bevy.org/) is written in Rust on `wgpu` and ships the same renderer to native and to the web (its `wgpu` calls compile to WebGPU in the browser). We keep our own engine, but phase 5 and the later fully shared renderer consult how Bevy handles the same problems before deciding:
- its render graph and pass structure;
- instancing and batching;
- asset loading (glTF);
- `winit` windowing and frame pacing;
- one renderer built for both native and WASM.

Cite what Bevy does in the research findings and wherever it shaped a decision. Bevy is an inspiration, not a dependency.

**Research first:**
- How TypeGPU-generated WGSL can be emitted for `wgpu`: entry points, bind group layouts, pipeline overrides.
- Which browser-only resources the passes use: canvas formats, timestamp queries.
- `wgpu`'s Metal backend support for the features the browser uses (reverse-Z, MSAA, storage buffers, indirect draws).
- Window and present pacing on macOS.

Write the findings into this file before building.

**Gates:**
- Plates match the browser's within the noise floor.
- Every counts gate holds (grass, instances, models, effects).
- **Performance:** native at or below the browser on every measure, both scenes, release build.
- **Less code:** Godot's project, its ported shaders and the GDScript go. Net code drops.

**Platforms** (user, 2026-10-11): native is verified on macOS only. Windows is not a requirement, but the native stack must not rule it out later. That is settled by research, not a Windows gate:
- `wgpu` runs on Windows through Direct3D 12 and Vulkan; Firefox and Bevy ship it there;
- `winit`, `gltf` and the chosen audio crate support Windows.

The research step confirms each dependency's Windows support and records it here. No Windows machine or Windows gate is needed.

**Then, the fully shared renderer** (user, 2026-10-11: liked, decided after measuring). `wgpu` also targets the browser's WebGPU through WASM. Once native `wgpu` is measured against TypeGPU, decide whether the browser renderer moves to the same Rust `wgpu` renderer, so both clients share one renderer and the host-side pass orchestration stops being duplicated. TypeGPU stays the browser's stack until then: it was chosen because it performs well (2026-09-27), and no earlier `wgpu` comparison is recorded.

**Order:** after phase 4's 19, 20 and 20b, because the 18b API is what both renderers consume. Planned in detail when phase 4 closes.

## Research (2026-10-11)

Probes are in `throwaway/phase5/`:
- `capture.mjs` wraps the WebGPU device in a page and records every WGSL module, pipeline, bind group layout, texture, query set and canvas configuration the browser creates. Run it under the GPU lock.
- `validate.sh` runs `naga` 30.0.1 over each module and translates it to MSL 2.4.
- `wgpu-probe/` is a scratch Rust crate on `wgpu` 30.0.1. It builds every captured pipeline on Metal from the recorded descriptors.
- `size.py` splits renderer files into host, shader and comment lines.

### The contract this proposes: one exported shader set

**Shared WGSL is a generated artifact.** The browser's TypeGPU passes emit, at build time:
- one `.wgsl` file per pipeline;
- one JSON descriptor list covering targets, depth, blend, MSAA, vertex buffer layouts and explicit bind group layouts.

Both are checked in beside 18b's `records.gen.*`. A staleness test fails if the browser's resolved text differs from the checked-in text. A Rust test runs `naga` over every file. Native loads the text and descriptors and mirrors nothing by hand.
- `capture.mjs` already records this descriptor list for all 70 pipelines.
- `wgpu-probe` already builds a native pipeline set from that list, in about 420 lines of Rust.

**Shared source is not recommended.** Hand-written `.wgsl` files that both clients import would mean rewriting about 3,900 lines of TGSL and template WGSL out of TypeScript, for no gain while TypeGPU stays the browser's stack. If the browser later moves to the Rust renderer (the fully shared decision), the WGSL files become the source at that point.

### 1. WGSL emission: every pass emits, and all 70 pipelines build on `wgpu` Metal

The capture ran two labs, `air` and `fog-look`; both draw through `battleFrame.ts`. The browser created 70 shader modules and 70 pipelines (60 render, 10 compute), with 16,499 lines of resolved WGSL:

| Family | Pipelines |
| --- | --- |
| Sky | LUT bake, background |
| Environment prefilter (PMREM) | 2 |
| World | prepass, shadow caster, opaque, water, terrain, backdrop |
| Models | prepass, caster, opaque, cutout ×3, rooms, blended, ghost, x-ray, x-ray count, cards |
| Fog compute | terrain march, merge, cull, probe, shape, whole |
| Scenery | 9 (forest, backdrop, dressing, felled; prepass, caster and colour) |
| Scar filter | compute |
| Grass | build, finish (compute); draw (indexed indirect) |
| Painted marks | 2 |
| Fog mask | rows, columns, compose, rim |
| Overlay | 5 |
| Effects | 1, hand-written WGSL through `tgpu.resolve({template})` |
| Post | 11 bloom levels, the bloom composite, and the AgX output in three modes (look, ungraded, raw) |
| Pose kernel | 1, hand-written WGSL, `layout: "auto"`; deleted if 18b's question 1 goes to Rust palettes |
| Impostor bake | 1 |

Shadows are the depth-only caster pipelines with no fragment stage. Haze and fog are the aerial function inside every lit module, plus the fog mask passes.

**naga.** 66 of 70 modules validate, and all 66 translate to MSL 2.4.
- **The one failure.** Tint accepts a function whose body ends in an infinite `loop` that only returns from inside; naga requires a return after the loop ("Returning None where Some(..) is expected"). Two functions do this:
  - `scarEntry` (`terrainMaterial.ts`), which fails the terrain colour and grass build modules;
  - `fogFoliage` (`fogVisibility.ts`), which fails the fog terrain march.
- **The fix.** One unreachable `return` after each loop, which does not change the browser's output.
- **Measured.** With that fix (applied textually in `wgpu-probe --patch`), **70 of 70 pipelines build on `wgpu` 30 Metal** (Apple M5 Pro), using the browser's own 29 explicit bind group layouts, vertex layouts, targets, depth state and 4× MSAA. Unpatched, 67 of 70 build.
- Earlier dumps (`throwaway/phase1-review/wgsl/`) show the same `scarEntry` failure; their other grass pipelines validate.
- The research agent found no other naga gaps that our shaders hit. The known ones are `ptr<storage|uniform|workgroup>` function parameters (`unrestricted_pointer_parameters`, [wgpu#5158](https://github.com/gfx-rs/wgpu/issues/5158)), `@diagnostic` on statements, and `enable subgroups;` ([wgpu#5555](https://github.com/gfx-rs/wgpu/issues/5555)). naga has no uniformity analysis ([wgpu#4369](https://github.com/gfx-rs/wgpu/issues/4369)), so it accepts more than Tint. That is why both validators run.

**What the text depends on.** No pipeline uses WebGPU pipeline-overridable constants: `constants` is unset on all 70 pipelines. Everything variable is spliced into the text or arrives through bind groups:

| Kind | Where | Today | On `wgpu` |
| --- | --- | --- | --- |
| Build-time tuning: terrain and grass tuning tables, workgroup sizes, scar bit masks, `CAST_FALLOFF` | terrain, grass, effects | spliced at module load; identical across both labs | the exported text, unchanged |
| Fog geometry from the fixture: `tile_px` (workgroup size), `radialBins` (a function-scope array length) | fog cull and merge | a function built per configuration | stays a text constant at export. WGSL allows override-sized arrays only in workgroup memory, so `radialBins` can't be an `override`. |
| **Per-battle light:** sun direction, sky radiance, aureole and transmittance, the aerial haze parameters | sky bake, background, and the aerial term in 15 lit modules | spliced per light. 17 modules differ between the two labs, and only in these literals. | **move them to a `Light` uniform**, which is 18b's `World.light`. Recommended (below). |
| **Per target size:** bloom texel offsets, e.g. `1/640` at 1280 px | post bloom (11 modules) | rebuilt on every resize | a small uniform; or keep the per-size rebuild, which works on `wgpu` too |
| Per-frame data: camera, cast lights, eyes, instances | every pass | bind groups | the same bind groups and buffers, laid out by the exported descriptors |

**Why the light should be a uniform.** In `wgpu-probe`:
- warm builds (the Metal shader cache already filled): the 70 pipelines take 0.57 s;
- the other lab's light makes 17 modules new: 3.8 s;
- the first run, cache cold: 5.1 s.

These are wall-clock figures and only indicative. They show that baking light into the text costs a Metal recompile on every new map, and that this cost doubles once both clients compile it. As a uniform, every map shares one compiled set.

TypeGPU 0.12.5 has no first-class `override`. Overrides could still be injected as raw declarations, but the uniform path needs nothing new, and it matches 18b's ownership: `World.light` is data.

**Bind group layouts.** Every pipeline uses an explicit layout except the pose kernel (`auto`). At most 4 bind groups are used, and at most 8 storage buffers in one stage (the water fragment shader). That is exactly WebGPU's default and well under Metal's 29.

### 2. `wgpu` features on Metal (30.0.1, released 2026-08-21)

Verified on this machine by `wgpu-probe`, and cited from source at the `v30.0.1` tag.

| Browser use | `wgpu` on Metal |
| --- | --- |
| Reverse-Z, `depth32float`, cleared to 0, compare `greater` / `greater-equal`, infinite far | Core. Bevy ships the same convention ([`projection.rs`](https://github.com/bevyengine/bevy/blob/v0.20.0/crates/bevy_camera/src/projection.rs#L339)). |
| 4× MSAA plus resolve on `rgba16float`, `rgba8unorm`, `bgra8unorm` | Core table ([`format.rs`](https://github.com/gfx-rs/wgpu/blob/v30.0.1/wgpu-types/src/texture/format.rs#L914-L991)). Probe: all three report `MULTISAMPLE_X4` and `MULTISAMPLE_RESOLVE`. |
| Multisampled depth read in the effects pass (`texture_depth_multisampled_2d`) | Core. The effects pipeline built. |
| Storage buffers read in vertex shaders, read-write and atomics in compute (grass arguments, fog) | Core. Metal also always offers `VERTEX_WRITABLE_STORAGE` ([`adapter.rs`](https://github.com/gfx-rs/wgpu/blob/v30.0.1/wgpu-hal/src/metal/adapter.rs#L1185-L1218)). |
| `drawIndexedIndirect` (grass, `grassPass.ts`) | Core. A nonzero first instance needs `INDIRECT_FIRST_INSTANCE`; this adapter has it. `multi_draw_indirect` is a CPU loop on Metal ([`command.rs`](https://github.com/gfx-rs/wgpu/blob/v30.0.1/wgpu-hal/src/metal/command.rs#L1589-L1620)). |
| `timestamp-query`: two marker passes, whole frame only (`gpuTiming.ts`) | `TIMESTAMP_QUERY` is true. `TIMESTAMP_QUERY_INSIDE_PASSES` and `_INSIDE_ENCODERS` are false on Apple GPUs ([`features.rs`](https://github.com/gfx-rs/wgpu/blob/v30.0.1/wgpu-types/src/features.rs#L702-L747)), which matches the browser's pass-boundary markers. |
| Formats: `rgba16float`, `rgba8unorm(-srgb)`, `rg8unorm`, `r8unorm`, `rg16float`, `r32uint`, `rg32uint`, `depth32float`; vertex `snorm16x4`, `snorm8x4`, `unorm8x4`, `uint8x4`, `uint16x2`, `float32xN` | All core. The probe built them. |
| `bgra8unorm` canvas, `alphaMode: opaque` | The Metal surface offers `Bgra8Unorm`, `Bgra8UnormSrgb` and `Rgba16Float` ([`adapter.rs`](https://github.com/gfx-rs/wgpu/blob/v30.0.1/wgpu-hal/src/metal/adapter.rs#L433-L444)). |
| Requested limits: 4 GB storage binding, 4 GB buffer, 2,048 array layers | The adapter reports 4,294,967,292 B, 4,294,967,295 B and 2,048. Request `adapter.limits()`, because `Limits::default()` is 128 MiB. |
| `@invariant` position (prepass and colour agree) | Supported from MSL 2.1 ([`msl/mod.rs`](https://github.com/gfx-rs/wgpu/blob/v30.0.1/naga/src/back/msl/mod.rs#L474-L482)). |

Only `TIMESTAMP_QUERY` and `INDIRECT_FIRST_INSTANCE` need a feature flag. Neither needs anything experimental.

**Present and pacing.**
- Metal offers only `Fifo` and `Immediate`, with no `Mailbox` ([`adapter.rs`](https://github.com/gfx-rs/wgpu/blob/v30.0.1/wgpu-hal/src/metal/adapter.rs#L462-L467)).
- `desired_maximum_frame_latency` sets `CAMetalLayer.maximumDrawableCount` to latency + 1, clamped to 1..2 ([`surface.rs`](https://github.com/gfx-rs/wgpu/blob/v30.0.1/wgpu-hal/src/metal/surface.rs#L330)).
- Recommended: `Fifo` with latency 2 for play, as Chrome and Bevy do.
- For timing runs, measure both modes on the first build step, because Chrome presents vsynced. Slice 07 found that a hidden window stalled Godot; whether `nextDrawable` throttles an occluded `CAMetalLayer` is **not yet measured**, and the bench minimises its window (slice 12).

### 3. Bevy as reference (0.20.0, 2026-10-08)

| Topic | What Bevy does | Take or leave |
| --- | --- | --- |
| Pass structure | Since 0.19 the render graph is gone: rendering runs as ordinary systems in named sets per camera, `Prepass → MainPass (opaque, then transparent) → EarlyPostProcess → PostProcess` ([0.18→0.19 guide, "Render Graph as Systems"](https://bevy.org/learn/migration-guides/0-18-to-0-19/), [`core_3d/mod.rs`](https://github.com/bevyengine/bevy/blob/v0.20.0/crates/bevy_core_pipeline/src/core_3d/mod.rs#L155-L166)). Extract, then prepare, queue, sort and render ([`bevy_render/src/lib.rs`](https://github.com/bevyengine/bevy/blob/v0.20.0/crates/bevy_render/src/lib.rs#L186-L231)). | **Take** the fixed ordered pass list and the extract-then-draw split. Bevy itself moved off a graph, and our `battleFrame` is already one function in order; `Frame` is our extract. **Leave** the ECS render world, schedules and system sets. |
| Instancing | `BinnedRenderPhase` for opaque work and `SortedRenderPhase` for transparent ([`render_phase/mod.rs`](https://github.com/bevyengine/bevy/blob/v0.20.0/crates/bevy_render/src/render_phase/mod.rs#L94-L109)). Per-instance data sits in a storage `GpuArrayBuffer` indexed by `instance_index` ([`gpu_array_buffer.rs`](https://github.com/bevyengine/bevy/blob/v0.20.0/crates/bevy_render/src/render_resource/gpu_array_buffer.rs#L22-L48)). GPU culling then expands it into `MeshUniform`s and issues indirect draws ([`gpu_preprocessing.rs`](https://github.com/bevyengine/bevy/blob/v0.20.0/crates/bevy_render/src/batching/gpu_preprocessing.rs#L150-L179)). | **Take** binning by key (ours is appearance, tier and pass, already 18b's `Batch`) and one buffer per batch key rewritten on version change. **Leave** GPU preprocessing and culling: Rust already culls batches, and `multi_draw_indirect` is a CPU loop on Metal. |
| glTF | `bevy_gltf` sits on the `gltf` 1.4 crate ([`Cargo.toml`](https://github.com/bevyengine/bevy/blob/v0.20.0/crates/bevy_gltf/Cargo.toml#L47)). | **Take** the `gltf` crate directly. **Leave** Bevy's asset server. |
| Window and pacing | `bevy_winit` on `winit` 0.30. `WinitSettings::game()` updates continuously while focused and at a reactive 1/60 s while unfocused ([`winit_config.rs`](https://github.com/bevyengine/bevy/blob/v0.20.0/crates/bevy_winit/src/winit_config.rs#L6-L105)). Present mode defaults to `Fifo` and latency to 2 ([`window.rs`](https://github.com/bevyengine/bevy/blob/v0.20.0/crates/bevy_window/src/window.rs#L326-L332)). Rendering is pipelined on a thread off WASM ([`pipelined_rendering.rs`](https://github.com/bevyengine/bevy/blob/v0.20.0/crates/bevy_render/src/pipelined_rendering.rs#L100-L140)). | **Take** `winit` 0.30's `ApplicationHandler` with continuous redraw, `Fifo`, and latency 2. **Leave** pipelined rendering until a profile shows CPU-bound frames. |
| Native and web from one renderer | One `wgpu` code path. The `webgpu` cargo feature maps to `wgpu/webgpu` ([`bevy_render/Cargo.toml`](https://github.com/bevyengine/bevy/blob/v0.20.0/crates/bevy_render/Cargo.toml#L31-L32)), plus a WebGL2 fallback. Shaders moved from `naga_oil` to WESL in 0.20 ([migration guide](https://github.com/bevyengine/bevy/blob/v0.20.0/_release-content/migration-guides/shaders_use_wesl.md)). | **Take** the single `wgpu` path (that is the later shared-renderer question). **Leave** WebGL2, which forces uniform-buffer fallbacks, and WESL/`naga_oil`: our WGSL arrives resolved. |

### 4. Windows

None of these rules out Windows.

| Dependency | Windows support | Source |
| --- | --- | --- |
| `wgpu` 30 | Direct3D 12 and Vulkan, first-class | [wgpu README](https://github.com/gfx-rs/wgpu/blob/v30.0.1/README.md#L66-L80) |
| `winit` 0.30.13 | Windows platform module | [docs.rs `winit::platform::windows`](https://docs.rs/winit/0.30.13/winit/platform/windows/index.html) |
| `gltf` 1.4.1 | Pure Rust; no C dependencies. Its last release was 2024-05, but Bevy still uses it. | [crates.io dependencies](https://crates.io/crates/gltf/1.4.1/dependencies) |
| Audio: **`kira`** 0.12.5 (recommended) | Built on `cpal`, which uses CoreAudio on macOS, WASAPI on Windows and Web Audio in WASM. It is a game-audio library (mixer, tweens, clock, spatial) and fits `Frame.emitters`. `rodio`, which Bevy uses, sits on the same `cpal`. | [kira `Cargo.toml`](https://github.com/tesselode/kira/blob/main/crates/kira/Cargo.toml#L24-L28), [cpal platforms](https://github.com/RustAudio/cpal/blob/master/README.md#supported-platforms) |

### 5. Host-side orchestration a Rust port must recreate

These are non-blank, non-comment TypeScript lines, with shader text (template WGSL and `"use gpu"` bodies) counted separately by `size.py`:

| Group | Host lines | Shader lines in TS |
| --- | --- | --- |
| **Pass orchestration** (wgpu recreates): `battleFrame` 355, `worldPass` 821, `sceneryLayer` 809, `grassPass` 582, `fogVisibility` 1,003, `modelLayer` 1,612, `impostor` 293, `post` 266, `effectPass` 193, and fog mask, overlay, scar filter, painted marks, targets, registry, timing, environment, sky, PMREM, shadow | **8,469** | 2,367 |
| CPU data prep that 18b, 20 and 20b move into Rust anyway: fog inputs, static chunks, detail view, scar packing, table packers, poses, placements, effect frame, launches, cook-offs, cast-light choice, light policy | 7,330 | 127 |
| Shared shader modules (`terrainMaterial` and `shaders/*`): mostly the tgpu function declarations around the WGSL | 955 | 1,409 |

Of the 8,469 orchestration lines:
- About 1,400 are declarative: struct schemas, bind group and vertex layouts, pipeline state. The exported descriptors replace them.
- The rest is buffer and texture lifetimes, uploads, the per-frame encode, stats, and lab toggles.

***Estimate:*** **4,000–5,500 lines of Rust** for native's renderer, with the shaders written zero times. `wgpu-probe`'s descriptor-driven pipeline builder (about 420 lines) already covers pipeline creation.

Against that, phase 5 deletes Godot's 11,315 GDScript lines, 1,691 shader lines and 1,191 lines of binding Rust. The "less code" gate holds with margin.

The cost is a second copy of the orchestration (TypeScript and Rust) until the fully shared renderer decision.

### Recommended build order

Each step is checked against the existing control plates and counts, scene by scene, as in phase 1.

1. **Make the WGSL portable, browser first.**
   - Add the two trailing `return`s.
   - Move per-battle light into a `Light` uniform fed from `World.light`, and the bloom texel sizes into a uniform.
   - Graphics-test plates must stay within the noise floor, and the browser frame time must not change meaningfully.
2. **The shader export contract.**
   - An exporter writes `<pipeline>.wgsl` plus the descriptor JSON.
   - A staleness test compares it with the browser's resolved text.
   - A Rust `naga` test validates every file.
   - This is the one place where the shaders exist once.
3. **Native skeleton.**
   - `winit` window, `wgpu` device with `adapter.limits()`, surface `Bgra8Unorm` with `Fifo` and latency 2, an `Immediate` switch for timing.
   - Registry, the sized targets (4× `rgba16float` HDR, the `rgba8unorm` mask, `depth32float`), two-marker GPU timing, pipelines built from the descriptors, and the 18b `load(World)` / `draw(Frame)` loop over a capture.
   - First picture: sky, haze and post on an empty frame. Measure present pacing and occlusion here.
4. **Ground:** terrain, shadows, scars, grass with its build, finish and indirect draw, painted marks. Plates against phase 1's controls.
5. **Models:**
   - `gltf` loading of appearances;
   - batches keyed by `Batch.key`, with buffers rewritten on `version`;
   - static and unit instances, palettes, impostor cards, scenery and felled trees.
   - Counts must equal the browser's.
6. **Effects, fog (compute and mask) and overlays** (overlays only if native gains UI). Slice 22's completeness items land here, on `wgpu`.
7. **Audio:** `kira` from `Frame.emitters`.
8. **Gates:** plates, counts, and performance at or below the browser on both scenes in a release build. Then delete `native/godot`, its shaders and GDScript, the Godot glue, and the parity harness's Godot runner.

## Steps 1 and 2: built (2026-10-11, branch `phase5a`)

### The contract

- **One shader text for every battle and every canvas size.** Nothing a battle or a size sets is spliced into WGSL any more:
  - the shown sky and the LUT bake read a `SkyLight` uniform (`shaders/physicalSky.ts`, packed by `skyLight`);
  - the haze reads `Environment.aerial`, an `Aerial` struct inside the environment uniform every lit pass already binds (`shaders/aerial.ts`, packed by `aerial`); it takes the sun from `Environment.sunDirection`;
  - post reads a `Bloom` uniform (threshold, knee, level weights, strength) in the highpass and composite, and each blur stage a `vec2f` step uniform. One blur pipeline per level now serves both axes (15 post pipelines became 10).
  - Each uniform field holds exactly the value the text used to hold. Where the text multiplied two literals, which the compiler folds, the packer folds them in f32 the same way (`cloudLit`, `cloudBase`).
- **Both validators accept every module.** `scarEntry` breaks out of its probe and returns the uniform word after the loop; `fogFoliage` is a `while` loop. A trailing unreachable `return` (the research's fix) passed naga but made Tint warn "code is unreachable" on three modules, so it was replaced.
- **Every pipeline has its own name** (`$name`): `worldPrepass`, `sceneryCaster`, `fogTerrain`, `grassBuild`, `bloomBlur2`, `postLook`, … Native looks pipelines up by these names. Two bind group layouts called `layout` are now `environment` and `scarFilter`.
- **The export** is `packages/battle-renderer/pipelines/`: one `<name>.wgsl` per pipeline and `pipelines.json` with each pipeline's kind, shader file, entry points, vertex buffers, targets, primitive, depth-stencil, multisample and bind group layouts by name (with their entries). 63 pipelines.
  - It is generated, never hand-edited: `web/tests/shaderExport.test.ts` builds the battle frame in Vitest on a recording WebGPU device (`web/tests/support/recordingDevice.ts`) and compares the resolved text and settings with the files. `UPDATE_PIPELINES=1` rewrites them. Vitest now runs TypeGPU through the same naming plugin Vite does (`server.deps.inline`), so the recorded text is the browser's byte for byte: the base's 68 modules matched the research capture of `/lab/air` exactly.
  - The same file proves the first point: the frame built under a second light (every haze, sky, cloud and bloom number different) at 1003×611 resolves to identical text and settings.
  - `crates/pipelines` runs naga 30.0.1 over every file: parse, validate, Metal Shading Language 2.4, and each listed entry point at its stage. Its capabilities are naga's defaults plus `SHADER_FLOAT16_IN_FLOAT32`, which `wgpu` grants every WebGPU-compliant adapter (`pack2x16float`).
  - Marked `linguist-generated` and left out of the formatter (`.prettierignore`).
- **Not exported yet:** the impostor bake and the pose kernel. Both are created only once appearances install, and the pose kernel's text depends on the installed joint count (slice 20 decides whether it survives). They are exported as they stand once slice 20 lands.

### Evidence

**Plates.** Six deterministic lab stills (`street`, `street` under a random sun, `street?sun=0.3`, `air`, `fog-look`, `ground`; each paused at a fixed tick, HUD hidden, 1280×720), two runs of the base and three of the branch, interleaved under the GPU lock (scratch: `throwaway/phase5a/`).

| Shot | base vs base (noise floor) | base vs branch (final code) |
| --- | --- | --- |
| air | identical | identical |
| fog-look | identical | identical |
| ground | 1 px, 1 level | identical, or 1 px at 1 level |
| street, `?sun=0.3` | 104 px, max 3 levels | 92 px, max 1–3 levels |
| street | 897 px, max 88 levels | 103–112 px, max 3–4 levels (or 900 px, max 88, against the other base run) |

Every difference is within the run-to-run floor of unchanged code. The branch logs no shader warning, and `fog-look`'s 8 TypeGPU buffer-size notices are the base's too.

**Compile time** (the same lab, a light this browser had compiled against one it had not; summed pipeline-creation time, then wall time from the first module to the last pipeline ready):

| | Known light | New light |
| --- | --- | --- |
| Base | 0.86–0.91 s; 6.2–6.6 s | **6.9–7.0 s; 7.8–7.9 s** |
| Branch | 0.80–0.96 s; 5.8–7.0 s | **0.82–0.86 s; 6.1–6.6 s** |

A new map's light no longer recompiles 17 modules; on the base it cost about 6 s of pipeline compilation and 1.5 s of wall time.

**Scenes.** `graphics-test` and `generated` (the battle) pass on the branch, including its frame-rebuild allocation baseline and "no WebGPU validation warning".

**Frame cost:** no meaningful change, under the GPU lock.
- *The same still frame, timestamp queries* (300 forced redraws per lab, base and branch interleaved, three rounds; the controlled measure): street 1.81/1.72/1.72 ms against 1.78/1.74/1.72; air 1.65/1.62/1.82 against 1.61/1.55/1.52; ground 1.29/1.33/1.41 against 1.26/1.25/1.32. The uniform reads cost nothing measurable.
- *Labelled Metal System Traces of Chrome's GPU process* (production builds, three interleaved runs per side, 3 s each): market-town's graphics-test reel 8 s into filming, GPU busy share 0.78/0.73/0.72 against 0.75/0.76/0.78; the city-contact benchmark's short run, 0.265/0.275/0.274 against 0.273/0.267/0.273. Busy per drawn frame moves with where in the reel or battle the 3 s land (reel 1.46/1.40/0.74 ms against 1.46/1.60/1.81, the 0.74 inside a fade; battle 3.21/2.02/2.74 against 2.54/3.16/2.36), so the still-frame timing above is the comparison that holds the frame fixed. One `xctrace` hung for 20 minutes holding the lock; it was killed and that run repeated.

**Tests.** `shaderExport.test.ts` (two tests), the naga test, `light.test.ts`, `fogVisibility.test.ts` and `scarShaderInlining.test.ts` pass. The full Vitest run's 9 failures (`nativeEffectVectors`, `nativePoseVectors`, `downloadBudget`, `unitFit`, `mapWorkbenchNative`, `battleFailure`, `navigationBrowser`) fail identically on the base.

**Code.** Production code +353 net lines (+764, −411) (TypeScript and the new crate, without tests, docs and the generated export); tests +368 (+375, −7). The export is 63 WGSL files and `pipelines.json`, about 0.9 MB, all generated. Most of the production growth is the three uniform structs and their packers (about 170 lines) and the naga crate (about 100); the pipeline names are one token each, but rewrap their statements. Nothing is deleted yet: Godot's ported shaders go at step 8.

### Decisions

- **The export runs in Vitest, not in a browser.** A recording device builds the real `createBattleFrame` in Node in about 0.2 s, so the drift check is an ordinary unit test. A browser capture would have made it a GPU-locked scene.
- **Light uniforms extend the owners that already bind data**, not a new `Light` binding: the haze rides in `Environment` (bound to every lit pass), the sky keeps its own small uniform, post its own. 18b's `World.light` is what will fill them.
- **Pipeline names are the native lookup key**, so every pipeline is named for its owner and role; the export refuses a missing or repeated name.
- **The blur's two axes share one pipeline per level**, since their text became identical.

### Next pickup

Step 3, the native skeleton, reads `crates/pipelines`'s directory and `pipelines.json`. Regenerate the export (`UPDATE_PIPELINES=1 bun run --cwd web test -- tests/shaderExport.test.ts`) after slices 20 and 20b land, and add the impostor bake and pose kernel then.

## Step 3: the native skeleton, built (2026-10-11, branch `phase5b`)

### The contract

- **`native/client`** is the native client: a library (the passes, the pipeline loader, the capture replay) and one binary that replays a parity scene and writes its plates. Its README holds the usage.
- **Pipelines come only from the export.** `Pipelines::build` reads `pipelines.json` with every struct `deny_unknown_fields`, fills WebGPU's defaults where the export leaves a field out (a `buffer: {}` is a uniform), builds each pipeline inside an error scope, and fails listing every pipeline the device refuses. Passes look pipelines and layouts up by name. All 63 build on Apple M5 Pro: 5.7 s cold, 0.06–0.08 s once Metal's shader cache is warm. `tests/pipelines.rs` is the gate (falsified once by breaking `bake.wgsl`).
- **Uniforms are presentation records, uploaded as bytes** (amends 18b):
  - `World.light` is `presentation::light::LightSettings`: the fixture's `presentation.light` resolved into uniform records, which the browser also reads (below).
  - `Frame.view` is 18b's `View` (view, projection, eye, size), filled every frame, even before the world loads, plus the sky's three corner rays (`sky_origin`, `sky_dx`, `sky_dy`, the browser's `frameCamera` rays). `cascades` and `sun_penumbra` wait for shadows (step 4).
  - `crates/pipelines/tests/uniforms.rs` holds each record to the struct the exported WGSL declares, member by member and in span, via naga (falsified by swapping two fields).
- **The frame is one ordered pass list.** `Passes { targets, passes: Vec<Box<dyn Pass>> }`, and a `Pass` has `name` and `encode(queue, view, targets, encoder)`. It is encoded in order each drawn frame, with a debug group per pass. Today the list is `[sky, post]`. There is no graph, no scheduling and no extract stage beyond `Frame` itself (Bevy 0.19+'s fixed order).
- **What draws.** The sky bakes its 384×192 lookup once at load, then each frame draws the shown sky (disc and clouds) along `Frame.view`'s rays into the 4× `rgba16float` target, resolved. Post is the browser's chain: highpass, five levels of two-axis blur, composite, then `postLook` (grade, AgX) into a `bgra8unorm` display target, which is copied to the window and read back for plates. The browser's fog step (lit → hdr) is skipped, since the reel draws no fog, so the sky resolves straight into post's input.
- **The replay drives the presentation crate through 18b's calls:** `Presentation::new(layout, tickHz)` and `load_world` (the manifest's `units` and `effects`, the clip tables, the light's sun, and the soldier's body and cruise height from `fixtures/game.json`). Then, for every frame record, every sample up to its tick is pushed, and `frame(simTime, camera from the record and the manifest's lens)` is called. The UI's x-ray and blocked colours are zeros, since a replay selects nothing.

### Evidence

Control `2026-10-11T18-47-00-317Z` (main checkout's `throwaway/native-parity/`), both scenes. Scratch is in `throwaway/phase5b/`: plates, contact sheets, `diff-market/`, `diff-paris/` and `critique/`.

- **Counts.** `Frame.counts` (soldiers, vehicles, aircraft, corpses) equal the manifest's `modelCounts` on all 24 plates. So the replay feeds the presentation crate what the browser fed it.
- **Sky region against the control.**
  - The top 140 rows of the market-town plates with no smoke or buildings in them: 280, 599, 961, 1385, 1821, 2226, 3384 and 3780.
    - Grayscale MAE 0.01–0.10.
    - No pixel off by more than 8 levels in any channel (most at most 3).
    - pixelmatch 0.
  - Paris-corner 424's gap between buildings (160×100 px): at most 3 levels.
  - The other market-town plates differ only where the browser draws smoke plumes or buildings into the band (effects and models are later steps).
  - By eye, cloud shapes, positions, shading, the blue gradient and the horizon tint match. The unprimed critique found no difference in the three market-town pairs. Its Paris findings were checked at 3× zoom and come from the downscaled side-by-side, not the plates.
- **Below the horizon** native shows the sky's horizon haze with faint vertical banding (the lookup's columns). The browser's ground and backdrop cover it.
- **`cargo test`** (native-client, presentation, pipelines) and **clippy `-D warnings`** are clean. `presentation-wasm` and `godot-binding` still compile.

### The window on macOS

`wgpu` 30's Metal surface checks `NSWindow.occlusionState` before `nextDrawable` (its workaround for [wgpu#8309](https://github.com/gfx-rs/wgpu/issues/8309)) and returns `Occluded` for a window that isn't visible. Measured over 60–138 paced frames:

| Window | Presented | Occluded | Frame interval p50 |
| --- | --- | --- | --- |
| Visible (on screen) | all | 0 | 8.33 ms (the 120 Hz display, `Fifo`) |
| Off-screen (borderless, at 10000,10000) | 0 | all | 0.55–0.61 ms, unpaced |
| Minimised | 0 | all | 0.54 ms |
| Hidden | 0 | all | 0.57 ms |

- **An occluded or minimised window presents nothing and nothing blocks.** Frames run unpaced, so plates never depend on presenting.
- **A timing run must change.** Slice 12's minimised bench measured Godot presenting. On `wgpu` the same bench presents nothing, so step 8's performance gate must compare GPU busy time (Metal System Trace or the two timestamp markers), or present to a visible window.
- **A live client needs its own pacing while occluded,** as Bevy throttles an unfocused window.
- **AppKit pulls a titled window back on screen.** The first runs asked for 10000,10000 and opened at 1040,66: about 3 s on the user's screen, three times, before the window became borderless. A borderless window stays off-screen.
- **Frames run from `about_to_wait` with `ControlFlow::Poll`,** not from redraw requests, which macOS doesn't deliver to a window it isn't showing.

### Code

The production code is +1,899 net lines:
- the client, 1,543;
- the presentation crate's light and view records, about 330.

The rest of the change:
- tests +206;
- docs +45;
- generated +2,567, almost all `Cargo.lock` (the `winit`/`wgpu` tree), plus `records.gen.ts` and the sky vectors.

This step only adds code. Godot's deletions come at step 8.

### Decisions

- **Rust resolves the light (18b's `World.light`).** The client derives nothing, and the browser reads the same records (below).
- **The sky rays go into `View`** rather than into each renderer: projection keeps one owner. They are an 18b amendment.
- **Present by copying the display target** (`COPY_DST` surface), so plates and the window read the same texture. A visible window costs one 1280×720 copy a frame.
- **The pass list is a `Vec<Box<dyn Pass>>`**, encoded in order. It has no render graph, as the research recommended.

### Not built yet (step 3's research list)

- the depth and fog-mask targets;
- two-marker GPU timing;
- the `Immediate` switch;
- resizing (targets are fixed at the capture's canvas).

These come with the passes that need them.
- **Follow-up for the export:** the sky's two layouts carry TypeGPU's automatic names (`bake - Automatic Bind Group & Layout`). Name them in `sky.ts` when the export is next regenerated.

### Next pickup

Step 4, the ground:
- terrain, shadows (fill `View.cascades`), scars, grass with its compute and indirect draw, and painted marks;
- add their passes to the list between the sky and post, merging the sky into the world colour pass as the browser does;
- plates against the same control.

## Step 3, web first: the browser reads Rust's light (2026-10-11, branch `phase5b`)

Set by the coordinator: a Rust contract lands in the browser first, so the light is resolved once.

### The contract

- **`presentation::light` resolves every uniform the light feeds:**
  - `SkyLight`;
  - `Aerial` (the haze);
  - `Sun`: direction, radiance (the atmosphere's transmittance greyed by overcast, times the intensity), the shadow floor and the fill, laid out as the environment uniform's members after its view;
  - `Bloom` and `Grade`.
  
  `LightSettings::records()` returns them all.
- **The browser takes them through `presentation-wasm`'s free function `light_records(json)`** (sky, aerial, sun, bloom, grade, in order). `resolveLight` in `web/src/battle/sim/presentation.ts` splits them into a `ResolvedLight` by the generated record sizes.
  - Not through `load_world`: the frame is built with its light before any battle loads, for labs with no battle, and per lab URL (`?sun=`). Rebuilding the frame when the world loads would rebuild its targets and pipelines.
  - So the light needs the module, not a `Presentation`. The function is stateless, so there is still one presentation per scene.
- **The renderer uploads the records as bytes.** `createBattleFrame`'s `light` is the `ResolvedLight`: the authored settings, which the cascades, the backdrop and the scenery still read, plus the five records.
  - The sky and post write their uniforms from the records.
  - The environment writes the `Sun` and `Aerial` records into its uniform once (at offsets from `d.memoryLayoutOf`) and only the view each frame.
- **Deleted from TS:**
  - `skyModelParams`, `transmittanceToSun` and the other derivations in `skyParameters.ts` (only the constants the sky's shader text splices remain);
  - `physicalEnvironment.ts`;
  - `aerialParameters.ts`;
  - the `skyLight` and `aerial` packers;
  - post's bloom and grade packers;
  - `postSettings`.

  `sunDirection` stays: the shadow cascades read it, and they are not records yet (step 4).
- **The shader text is unchanged.** The export test passes without `UPDATE_PIPELINES`.

### Evidence

- **Equal values.**
  - `crates/presentation/tests/light.rs` holds sky, aerial and sun to the browser's last values, on the game's light and on a low sun under half overcast. `tests/fixtures/light-vectors.json` was written by the TS before it was deleted.
  - Each record is falsified once.
  - A scratch sweep (`throwaway/phase5b/equal/`) ran the base's TS against the branch's WASM over the fixture's light, `?sun=0.3`, a sun at 0.12 and 60 seeded variants of every field. All 63 lights are equal word for word at authored precision.
  - With 17-digit random inputs, 7 of 63 lights differ by one f32 ulp in one directly copied field. `serde_json`'s default float parse is not correctly rounded. Its `float_roundtrip` feature would fix that, but feature unification would also change how `sim` parses its fixtures in some builds and not others. So it stays off.
- **Behaviour tests moved to Rust:** one sun for the sky and the ground, the shadow floor kept, finite non-negative light from horizon to zenith and clear to overcast, and clouds giving way to the overcast dome. `crates/pipelines/tests/uniforms.rs` holds `Aerial` and `Sun` to the WGSL structs.
- **Scenes** (base `34afc4556` and branch, same machine, under the lock). `fog-look` and `air` pass the same 18 checks on both, and fail the same 4, which already fail on the base and are not light:
  - units standing in fog are never fogged;
  - the air lab's rifle and autocannon hits (three checks).

  `graphics-test` passes.
- **Plates.**
  - Every fog-look `-world`, mask, glyph and whole-building frame is byte-identical, base to branch (32 frames).
  - The full frames differ in about 1,000–1,700 scattered single pixels on the grass. A diff map shows only grass speckle; the browser's wind runs on wall time. The air frames differ in the same way.
  - The sky, the buildings, the shadows and the haze are identical.
- **Quick reel** (`graphics-fps`, paris-corner, production, headless, three interleaved runs per side). The light is set once, when the frame is built, so per-frame work is unchanged except that the environment write is now raw bytes.

  | | avg FPS, median (min–max) | frame ms p50 / p95 | GPU ms mean |
  | --- | --- | --- | --- |
  | Branch | 93.2 (79.6–103.5) | 8.34 / 16.81 | 6.57 |
  | Base | 82.8 (76.6–102.8) | 8.48 / 17.12 | 6.74 |

  No regression; the spread is the headless pacing.
- **Code** (this pass): production −115 net. That is TypeScript +136 −432 and Rust +189 −8 (the light's derivations, the two records and the binding). Tests +187.

### Decisions

- **A free `light_records`, not `load_world`.** See the contract above. 18b is amended.
- **Native Godot's own ports of these formulas** (`environment/light.gd`, `sky_model.gd`, `aerial.gd`) are frozen and leave with Godot at step 8. Their comments name the deleted TS.

## Step 4: the ground, built (2026-10-12, branch `phase5g`)

Native draws the terrain, the backdrop and the map's skirt, the sun's cascades, the side's scars, the grass, and the water, through the browser's exported pipelines. It also draws the buildings. Every number it uploads is the presentation crate's. Three pieces the browser still derived in TypeScript moved into Rust first, browser first: the cascade fit, the ground's meshes with the grass field's CPU side, and the learned ground with its scar cache. Two lanes built them in parallel (`worktree-agent-acad722b…`, `worktree-agent-a9cfa9d2…`), merged here.

### The contract

- **The sun's shadow is Rust's** (`presentation::shadow`, browser first). `sun_shadow(light, camera, map_box)` fits the frame's cascades and returns a `SunShadowFrame`: the receiver block and one caster camera per active cascade.
  - Two new records: `SunShadow` (the shaders' receiver block, 400 B) and `Camera` (the camera uniform, 160 B). `crates/pipelines/tests/uniforms.rs` holds both to the exported WGSL.
  - It covers the receiver range over the map's box (`map_box`), the splits, the texel-snapped fits, the biases, the PCF radius and the inactive and cold records.
  - `LightSettings` gains `cascades` and `backdrop`.
  - **`view::Camera`'s `up`, `fov_y` and `near` are now `f64`.** In `f32`, the fit's texel-snap floors flipped against the browser's.
  - The page calls the binding's `SunShadow` once a frame and uploads the words. `cascadePolicy.ts`, `shadowData.ts`, `receiverRange.ts` and `shadowPolicy.ts` are deleted.
- **The ground's meshes are Rust's** (`static_world::ground::mesh`): `terrain` (with the traversal overlay's tints), `skirt`, `water` and `backdrop`, as world vertices. `backdrop.ts`, `waterSurface.ts`, `addSkirt` and the terrain loop in `terrainSurface.ts` are deleted.
- **The grass field's CPU side is Rust's** (`static_world::ground::grass`):
  - `pack_shapes`, from each grass kind's tier meshes: the bundle's arrays, which each client reads itself;
  - `strip_indices`, `blade_vertices`, `FIELD_SEGMENTS` and `LIMITS`;
  - `Ground::grass_window` and `frame_params`, the per-frame window and the `GrassParams` fields.

  The renderer keeps only its regrow check: it regrows when the view changes.
- **The side's learned ground and its scar cache are Rust's** (`presentation::scars`, browser first). Each publication's ground patch is applied in `push_publication`. A renderer calls these on `Presentation`:
  - `scars_changed()`, after which the grass regrows;
  - `scar_regions()`, empty when the marks fit at once;
  - `prepare_scars(region, clipped)`, which returns a `ScarUpload`: the directory's words and size, the pool's texel runs, and `ScarParams`' first 20 words;
  - `resend_scars()`, for a renderer that holds nothing;
  - `set_scar_reach(Ground::scar_reach_m())`, the halo.

  The page's `ground.ts`, `groundPage.ts`, `groundHoles.ts`, `ScarSync` and `scarPool.ts` are deleted.
- **Native reads the ground from the repository's own files.** `static_world::files::ground(root, map)` builds the map with the simulation's world code under the game's rules, in the map's biome (`meta.json`) and its regional finish. It is the same `Source` the page builds from `WorldView`. The layout's prop-kind parsing moved from the WASM binding into `ground::drawn_by`, so both clients use one owner. `files::placed_buildings` stages the buildings, with each building's parts and the shell kinds the manifest carries.
- **Native's frame:** `sun-shadow → depth prepass → world → effects → fog mask → ghosts → post → fog rim`.
  - **The sky is merged into the world pass**, as `worldPass.encode` does. The pass grows the grass (compute), draws the sky, then the terrain, the skirt, the models, the grass, the backdrop, the water, and glass last.
  - **The scars load at the browser's submission points.** The grass window's upload comes before the build. Then comes either the view's upload, or, region by region, an upload and a submitted terrain pass for each.
  - **The depth prepass** draws the ground before the models.
  - **Each cascade** draws the ground's casters and the models' casters. A unit casts with the tier one coarser than it draws, and never as a card, as `drawCasters` does. The static world casts only in its caster batches.
  - `Passes::prepare` now takes the frame's camera, and `prepare_scars` takes the presentation. `Inputs` gains the device, the shadow, the ground and the grass.

### What draws, and what is left

- **Draws:**
  - the terrain, with its plots, rows, wheelings, roads, kerbs and markings, the forest floor and the river banks;
  - the skirt and the backdrop past the map;
  - the four cascades, cast by the ground, the units, the corpses, the wrecks and the buildings;
  - the scars: tracks, craters, scorch and trampling, in the ground and in the grass;
  - the grass, with its build, finish and two indexed indirect draws;
  - the water, through the exported `worldWater`. No reel map has water, so no plate judges it.
- **The buildings, a small addition:** they are the frame's static batches. A replay frames only the plates it draws, so on those frames it asks `resend_statics()`; a frame whose walk changed nothing names its static batches `first = NONE`. Building-module counts equal the control's on every plate.
- **Left, and why:**
  - *Scenery* (trees, hedgerows, dressing): the `slice20b3` lane is moving the scenery walk into Rust.
  - *The map's props and the known props* (parked cars, lamps, fences, walls, bins, resting wrecks): the static world draws them with the scenery, not through `Frame`.
  - *Painted marks*: the UI paints them, and a replay has no UI. The paint target stays all zeros, which is the browser's own state in the reel. Nothing would exercise a painted-marks pass yet.
  - *Fog of war's sight*: as in step 6.
  - *The per-region scar path* is built but not exercised by either reel: every reel frame's marks fit at once, 69 to 437 pages on market-town.
  - *Offscreen scar regions are not skipped.* The browser projects each region and skips the ones off screen; native draws every region, which draws the same picture. Moving that cull into Rust is for when a battle needs it.

### Evidence

Control `throwaway/native-parity/2026-10-12T10-11-20-573Z`, captured from this branch's merged tree: the browser reads Rust's fit, meshes, grass field and scars. Native plates, pairs (control on top) and crops are in `throwaway/phase5g/` (`n2/`, `cmp/`, `critique/`).

- **Counts:**
  - Models and building modules equal the control's on all 24 plates.
  - Effects are equal on 22 of 24. Market-town 166 and 2216 differ by step 6's tick-boundary rule (tracers 77 against 51 on 166, one puff on 2216).
- **Plates** (`compare-screenshots`, every plate):

  | | grayscale MAE | pixelmatch ratio |
  | --- | --- | --- |
  | market-town | 0.72–3.48 | 0.008–0.059 |
  | paris-corner | 0.005–4.55 | 0.00004–0.073 |

  Before the ground (the step 6 state against the earlier control), market-town was 2.5–14.1 MAE. The absolute-difference maps of the worst plates (market-town 2216, paris-corner 163) are lit only on the trees, the fences, the cars and the lamps, and on those objects' shadows.
  - At full size, the tank's tracks on the gravel road behind it (market-town 864), the tank's soft shadow, the grass clumps and their colour, the field rows, and the Paris street's markings and building shadows sit where the control's do.
- **Unprimed critique** of all 24 pairs and five crops. It was told only that the two halves are two renderers of one moment.
  - It found the terrain, field rows, grass, shadows (no acne or banding), roads, tracks and craters, and the Paris streets the same.
  - It found every object difference in the out-of-scope layers: trees, resting wrecks, street props, walls and fences.
  - Its one open ground question (green in town against grey pavement) was checked at 2×: the pavement is the same in both, and the green is trees, hedges and park furniture.
  - It rated the grass "very slightly softer" in B at low confidence.
- **Tests:**
  - `cargo test` passes for `presentation`, `pipelines` and `native-client` (under the lock). Clippy `-D warnings` is clean on those three and on `presentation-wasm`.
  - The web typecheck is clean on the merged tree.
  - The lanes' own checks are below.
- **The lanes:**
  - *Shadow, meshes and grass:* every output equals the deleted TypeScript bit for bit, with no ulp differences, on vectors dumped from it first.
    - `tests/shadow.rs`: 136 fits, giving the receiver blocks and 544 caster cameras, plus the cold blocks. They cover both reel maps' boxes, ground-level to 6 km cameras, pitch from 0.12 to vertical, two lenses, three viewports, three suns, and fits with no box.
    - `tests/ground_meshes.rs`: the terrain, the traversal overlay, the skirt, the water, the backdrop and the map box, on six maps built natively through `sim`.
    - `tests/grass_field.rs`: the shapes and facts of all 10 grass bundles, and 80 windows with their frame words.
    - Each test was falsified once.
    - `view.rs` and the sun direction now use `libm`'s trig, so both clients compute the same values.
    - `graphics-test` and `ground` pass. The `street` scene fails 3 checks: one fails on the base too, one compares an f32 half extent with 0.3 exactly and cannot pass on either side, and one looks like battle timing (not confirmed on the base, whose run timed out).
  - *Browser frame time, shadow, meshes and grass:* six runs a side, interleaved under load (load average 7–22). Market-town is level: 24.3 against 23.8 FPS, GPU 23.5 against 26.1 ms. Paris-corner's median is 29.2 against 35.1, but single runs swung from 20 to 56 FPS, so this neither shows nor rules out a change of about ±15%. The new per-frame calls cost 0.019 ms in a micro-benchmark, and every GPU input is bit-identical.
  - *Scars:* Rust's uploads equal the deleted TypeScript's word for word over both reels' publication streams (1,291 and 3,537 publications, including about 200 resend and hide/show cycles) and over synthetic streams that force epochs, holes, pool compaction and the regional path (1,300 region uploads). `tests/scars.rs` replays the vectors and reads every learned cell back through the directory and pool. Thirteen planted bugs each broke it. `ground`, `graphics-test` and 57 web tests pass.
  - *Browser frame time, scars:* six interleaved runs a side, under machine load (load average 9–24 from other sessions). Market-town 25.4 against 24.3 FPS, and Paris-corner 31.9 against 32.3; both inside the noise, so no regression detected rather than a measurement. Over market-town's publications, the scars' CPU work fell from 195–254 ms to 81–118 ms.

- **Native's run-to-run floor.** Two replays of unchanged code are byte-identical on Paris-corner, which has no grass. On market-town they differ in 39–2,840 scattered pixels a plate. Most are 2–18 levels, and one plate (1777) has a pixel at 44. The grass build appends clumps through atomics, so their order, and with it which blade wins a depth tie, varies from run to run. After the camera uniform moved to Rust's `camera_record` and origin/main was merged, the plates sit within that floor.
- **After merging origin/main** (`7dc5b5b72`):
  - `cargo test` passes for `presentation`, `pipelines` and `native-client` (under the lock). Clippy `-D warnings` and `cargo fmt --check` are clean on those three and on `presentation-wasm`.
  - The typecheck and the 8 affected web test files (51 tests) pass.
  - Model and building counts are still equal on all 24 plates, with effects as above.
  - Main's own `native/client/tests/audio.rs` did not compile, because main added `Heard.shouts`. It is fixed here.
- **Browser frame time, the merged step** (`graphics-fps`, production, headless, three interleaved runs per side under the lock, the pre-step main `494071e7c` against this branch; load average 4.5–13 from other sessions). Market-town averaged 25.1, 23.8 and 20.2 FPS against 24.5, 25.4 and 21.9, with GPU ms medians of 25.8 against 25.3. Paris-corner averaged 32.5, 35.0 and 21.7 FPS against 34.8, 34.4 and 27.9, with GPU ms medians of 20.7 against 21.2. Each adjacent pair is level within the run-to-run swing. The third pair ran at load 13 and moved both sides. The GPU time is unchanged, as bit-identical inputs predict. These are not quiet-machine numbers.

### Code

Native and the loader (this lane, without the merges): Rust +1,492 −438, net **+1,054**:
- `ground.rs`, `grass.rs`, `shadow.rs` and `scars.rs`;
- the world pass's order and its scar submissions;
- the models' casters;
- `files::ground` and `placed_buildings`.

Docs: the native client's README.

The lanes (comments and blanks excluded):

| | Rust | TypeScript |
| --- | --- | --- |
| Shadow, meshes, grass: production | +1,011 (`shadow.rs` 424, `grass.rs` +233, `mesh.rs` 142, bindings +154) | −609 |
| Shadow, meshes, grass: tests | +631 | −199 |
| Scars: production | +1,160 | −933 |
| Scars: tests | +706 | −733 |

Generated: `records.gen.ts` +24. Fixtures: 1.36 MB of vectors JSON (shadow, grass, mesh digests, scars).

Production code is net +2,329 lines over the step: Rust +3,225 against TypeScript −1,542, with native's Godot-free ground at +1,054. The Rust is longer than the TypeScript it replaced: rustfmt's layout and the WASM marshalling account for most of it. Godot's deletions at step 8 settle the "less code" gate.

### Decisions

- **Three moves went browser first in parallel lanes**, each with its own equality vectors and FPS pair, then merged. Each lane deleted the TypeScript it replaced.
- **Native draws every scar region** rather than projecting a scissor and skipping regions off screen (`worldPass.ts`'s `scissor`). The picture is the same, and the region path is cold in both reels.
- **The replay asks for every static batch on each frame it draws** (`resend_statics`), because it draws only the plate frames. A live client draws every frame and keeps its last static upload, as the browser does.
- **The skirt draws through `worldOpaque`** with the same all-zero fog group as every other layer, since native has no fog of war.
- **`view::Camera` is `f64`** (an 18b amendment), so the cascade fit matches the browser's bit for bit.

### Next pickup

- Re-measure the browser's frame time on a quiet machine: every pair this step was run under other sessions' load.
- The scenery lane (`slice20b3`) brings the trees, the hedgerows and the dressing through `Frame`. The map's props and the known props come with it or next. Native's model layer already draws any static row.
- The painted marks and fog of war's sight come once native has a UI and fog inputs.

## Step 5a: the models, built (2026-10-12, branch `phase5c`)

Native draws every unit, corpse, ghost and moving wreck piece a `Frame` lists, through the browser's exported model pipelines. The ground (step 4) is not built, so the models stand over the sky's horizon haze; no placeholder ground was needed to judge them.

### The contract

- **The frame's pass list** is now `sky → depth prepass → world → ghosts → post`, the order `battleFrame` encodes its world (`worldPass.ts`'s `encodeDepth`, `encode`, `encodeGhosts`). The ground and scenery join these same passes at step 4.
  - The depth prepass (reverse-Z, cleared to 0) draws `modelPrepass` over the solid parts and `modelPrepassCutout` over the cutouts.
  - The world pass loads the sky, resolves into post's input, and writes the fog mask beside it (a new multisampled `rgba8unorm` target, never resolved until the fog mask passes exist). It draws `modelOpaque`, `modelCutout`, `modelRooms`, then `modelBlended`.
  - The ghost pass draws `modelGhost` over the finished world, reading depth and writing none, only when a frame has ghosts.
- **Each `Frame` first prepares the layers** (`Passes::prepare`): the environment's camera and view, then the models' palettes and instances, written whole. A pass then reads `Inputs` (queue, view, targets, environment, models). The `Pass` trait changed from `encode(queue, view, targets, encoder)` to `encode(inputs, encoder)`.
- **Appearances load from the browser's bundles, not glTF** (`bundle.rs`). The browser never draws glTF: the bake turns sources into `assets/runtime` bundles, and that is what both clients read. The `gltf` crate the research proposed is not needed. Native reads the runtime catalog, the gzip transport, the `BGAT` container (JSON header, typed views) and each texture file, as `scene-assets`' `gzip.ts`, `codec.ts` and `texture.ts` do, and refuses an LFS pointer with the pull that fetches it.
- **Every `World.appearances` row is loaded once** (`models.rs`), into the browser's 48-byte vertex (`packVertices`, ported with `Math.round`'s halves), with each tier's indices ordered by surface class (`orderSurfaces`), materials as the browser's five rows, and textures in the two arrays (`uploadTextureArray`). A batch is one run of the frame's instances over one row's tier, keyed by its row; a batch with `first == NONE` is skipped, as in the browser.
- **Rest palettes come from Rust** (amends 18b): `Presentation::rest_palette(row)` is a skinned row's body as it holds still, a corpse row as it lies and a body row in its far pose. Native skins the row's mesh by it once at load (`posedMesh`), and the frame's instance takes the identity, as before. The browser still poses its corpses in TypeScript (`poseWorlds`); moving it onto `rest_palette` is a later cleanup.
- **The environment** (`environment.rs`) owns the frame-level groups by layout name: the camera (`viewProj`, its inverse, eye, near, size, clock, from `View`), the environment light (the world's `Sun` and `Aerial` records, the sky lookup, the PMREM prefiltered from it as `pmrem.ts` does with the exported `pmremBake` and `pmremFilter`, and the DFG table read from the browser's `dfgLut.ts`), and fog.
  - Bound, but read as absent by every shader: the sun's cascades (every interval below zero, so no receiver is inside one), the cast lights (a zero count) and fog of war (every word zero: `enabled` 0, nothing painted).
- **A pass binds groups by the layout they fill** (`Pipelines::bound`, from each pipeline's exported layout list), not by number.

### Next pickup

- Step 4, the ground, adds its draws to the depth prepass and world passes, fills `View.cascades` and the sun-shadow binding, and adds the model casters (`modelCaster`, `modelCasterCutout`) into the cascades.
- The statics (buildings, scenery, props, resting wrecks) come through `Frame` batches once slice 20b lands; the model layer already loads any static row and draws its state.
- Bake impostor cards once the impostor bake is exported, and replace the far-pose stand-ins.

### Decisions

- **Impostor cards are not baked yet** (the impostor bake pipeline is not exported). At the impostor tier, a body draws its coarsest mesh posed in its far pose, which is what its card shows, and a corpse its coarsest lying mesh. Counts hold; a far soldier is a few hundred triangles instead of two.
- **No model shadow casters yet.** The model pipelines need a sun-shadow binding, not casters; cascade fitting (`View.cascades`) belongs to step 4's shadows, and the casters join then.
- **The cast lights stay empty** until the effects step feeds `Presentation::lights()`.
- **A wreck piece the bundle lacks** (a helicopter wreck has no turret to throw) is a row nothing draws, as the browser's `states.get` returns nothing.
- **A fresh control.** The control the skeleton used (`2026-10-11T18-47-00-317Z`) predates slice 20's manifest fields: its units carry no `bounds` or `tiers`, so every model's drawn size was zero and the presentation crate tiered every soldier and corpse as an impostor and every vehicle at tier 3. Its counts still matched, because counts don't depend on size. The new control is `throwaway/native-parity/2026-10-12T06-54-57-178Z` (this worktree's scratch), captured from this branch's tree. Its manifest also carries `wreckLooks`, so cook-off rows (73 wreck appearances) now load too.

### Evidence

Control `2026-10-12T06-54-57-178Z`, both scenes. Scratch: `throwaway/phase5c/` (`n-*` native plates, `p-*` cropped pairs with the control on top, `critique/`).

- **Counts.** On all 24 plates, `Frame.counts` (soldiers, vehicles, aircraft, corpses, fading) equal the control's `modelCounts`, and native draws every instance the frame's batches list (for example market-town 2777: 5 soldiers, 3 vehicles, 35 corpses and 1 wreck piece of 44 listed; paris-corner 1420: 27 soldiers and 1 corpse of 28).
- **Plates against the control** (`compare-screenshots`, pairs cropped to where native drew models, on silhouette, pose and placement):
  - Paris-corner 359's Abrams matches the control in outline, its stowage, rails, antennas, turret and gun, the track and its textures, at the same pixels.
  - Paris-corner 1420's squad matches man for man: the same standing, aiming, crouched, leaning and prone poses, rifles and kit, at the same places.
  - Market-town 2777's corpses lie where the control's do, and its tanks and soldiers stand where the control's do.
  - Native also draws the units the control hides behind buildings, bollards, grain and smoke. That is expected: those layers come later.
  - Lighting differs (no ground bounce, shadows, haze from effects); that is out of scope for this step.
- **Unprimed critique** (all 24 pairs and four zooms, no history): every living unit and every soldier corpse matches in position, size, pose, facing and turret and gun angle; no holes, stretching, z-fighting, wrong textures, or floating or sunk units.
  - Its one high-confidence finding: **resting vehicle wrecks are missing** (market-town 3415, 3702, 3965, 4229, 4502, 4791). A wreck at rest is not a `Frame` batch: once its cook-off settles, the side knows it as a prop (the control's "known props per plate", `us_m1_abrams_sep_v2_wreck`), drawn by the static world with the buildings and scenery. It arrives with the statics, not here. Moving pieces during a cook-off are batches, and native draws them.
  - Its low-confidence notes (one corpse looking more sprawled where the control's grain hides its legs; one leaning soldier's boots a pixel or two lower) were checked at zoom: grass occlusion and resolve noise.
- **Load time.** All 367 market-town rows (27 unit appearances, their corpses, and 73 wreck looks' pieces) install in 3.2 s in a release build (49 s in a debug build).
- **Tests.** `tests/models.rs`: a corpse row's posed body lies on the ground (falsified by posing at the bind pose: the body stood 1.71 m), and a far body stands in its far pose. `cargo test` for `native-client` (with the pipelines test under the GPU lock), `presentation` and `pipelines`, and clippy `-D warnings` on `native-client` and `presentation`, are clean. `presentation-wasm` and `godot-binding` still compile.

## Step 6: effects and fog's mask, built (2026-10-12, branch `phase5e`)

Native draws every effect instance and cast light a `Frame` makes, through the browser's exported `effects` pipeline, and runs fog's mask passes as `battleFrame` does. Fog of war's sight (the compute) is not built: its inputs are the static world's.

### The contract

- **A fog look is a presentation record** (web first). `presentation::fog` reads `presentation.fog` (`FogPresentation`, `FogStyle`), checks a style (the ranges the TS checked) and resolves it into `records::FogLook`, the fog mask passes' `FogStyleUniform`: the tint at unit luminance, the line angle in radians, the search reach. `crates/pipelines/tests/uniforms.rs` holds it to the struct in all four mask shaders (falsified by swapping two fields).
  - The page takes it through `presentation-wasm`'s free `fog_look(styleJson)` and `resolveFogLook(style)` (`web/src/battle/sim/presentation.ts`), which throws naming the field out of range.
  - The renderer takes words, not a style: `BattleFrameOptions.fogStyle: FogStyle` is now `fogLook: Float32Array`, and `BattleFrame.setFogStyle` is `setFogLook`. The mask pass reads its edge and rim switches from the record (`FogLook` offsets in `records.gen.ts`).
  - Deleted from TS: `fogStyleUniform`, `validateFogStyle`, `validateFogPresentation`, `selectedFogStyle`. `FOG_EDGE_REACH_PX` is generated into `records.gen.ts` from `fog.rs`; `FOG_DISTANCE_CAP_PX` stays in `fogStyle.ts`, where the shaders splice it.
  - `gameFog.ts` no longer checks the fixture's fog block at import; a style is checked when it is resolved (the frame build, the fog-look lab's edits).
- **Native's frame** is `sky → depth prepass → world → effects → fog mask → ghosts → post → fog rim`, `battleFrame`'s order.
  - New targets: `lit` (the world resolves into it, and the effects draw over it), the resolved `fog_mask`, and the two `rg8unorm` distance targets. `hdr` is now the fogged world, post's input. Depth is sampled (the effects read it).
  - `Passes::prepare_effects(presentation, light_wrap)` uploads `Presentation::effects()` as the instance buffer and `Presentation::lights()` into the environment's `CastLights` (count, wrap, rows), which every lit material and the smoke read.
  - The flipbooks are the browser's sheets, read by name from `flipbooks.ts` (as the DFG table is read from `dfgLut.ts`), premultiplied in their encoded bytes as `createImageBitmap(premultiply)` does, into a mipmapped `rgba8unorm-srgb` array. Mips are a 2×2 box chain; the browser resizes each level from the sheet with Chrome's "high" filter.
  - The fog mask passes are built from the replay's look (the fixture's selected style): the distances only when the look has an edge, the rim only when it has one, the compose always.
- **The replay reports effect counts beside model counts**: per plate, the running effects by kind, the lights, the instances by shape, and the instances drawn of those listed, against the manifest's `effectCounts`.

### What draws, and what is left

- **Draws:** tracers, glows, flashes, fireballs, smoke and dust flipbooks lit by the sun, the PMREM and the cast lights, smoke trails, sparks, rotor blurs (none in either reel), and the cast lights on every model material. Slice 22's lit smoke from the prefiltered environment and cast light following normal maps are drawn by the browser's own shaders here.
- **Already on `wgpu` since step 5a:** side-colour tints, wear chips and the cook-off's moving wreck and thrown debris (all model rows the `Frame` lists).
- **Left, and why:**
  - *Fog of war's sight* (`fogTerrain`, `fogMerge`, `fogCull`, `fogProbe`, `fogShape`, `fogWhole`). Its input (`FogInput`: the terrain grid, foliage, the map's occluding props less those the side saw fall, and the eyes) is assembled in TS (`fogInputs.ts`, `useBattleSession`) from the static world. It needs slice 20b's static world in Rust and step 4's terrain, then a web-first move of `fogEyes`, `mapOccluders`, `knownOccluders` and `wholeWords` into `crates/presentation`. Neither reel draws fog (the graphics test hands the viewport no fog), so the browser's control runs no fog compute either and the mask is empty: native's fog-off bind group is the same state.
  - *Emissive lamp glass, lit interiors, leaf clumps, dressing shrink, railing dither, water in shadow* belong to the static world and the ground (slice 20b and step 4).
  - *Overlays and the fog-mask lab views:* native has no UI.

### Evidence

Two controls, both captured from this branch's tree: `throwaway/native-parity/2026-10-12T07-29-29-056Z` before merging main `356a0b7f4`, and `2026-10-12T08-04-37-942Z` after. Native plates, pairs (control over native) and crops are in `throwaway/phase5e/`. The plates were judged on the first; the counts were checked on both.

- **Counts.**
  - Models: equal on all 24 plates, and every listed instance drawn, as in step 5a.
  - Effects: native draws every instance the frame lists on all 24 plates, on both controls.
  - **Every plate whose sim time falls between ticks is equal to the browser's**: 19 of 19 on the first control, 10 of 10 on the second.
  - **Plates drawn exactly on a tick boundary** (`simTime × 30` an integer to within 1e-13) mostly differ. On the first control 5 of 5 differ (market-town 239, 1203, 1574, 5195, 5435). On the second, 12 of 14 differ. How many plates land on a boundary varies from run to run (5 of 24 on the first, 14 of 24 on the second). The headline is therefore 19 of 24 equal and 13 of 24 equal.
  - **It is the replay's input, not native.** `throwaway/phase5e/replay.mjs` replays the same capture through the browser's own WASM module on the same calls, and gets native's counts on every plate, including the same five differences.
  - Effects are a pure function of the publications noted, the time and the camera: framing only the plate records changes nothing.
  - At a boundary the effect set is discontinuous: launches from tick `k + 1` start at exactly `k / 30`. No publication set from `k − 2` to `k + 2` at times within ±1e-9 reproduces the browser's set. The capture records the sim's tick and the clock, not which publications the page's presentation had noted when it framed. The worst plate (239) has 96 streaks against 72 and 11 glows against 6; the rest are within a few instances.
- **Plates** (`compare-screenshots`, pairs and full-size crops; target: the same effects in the same places, looking the same, wherever native draws them).
  - The beaded rocket trail (market-town 1918), the dust behind each tank (239, 2392), the burning wrecks' columns and base fires (847, 3445–5195), the tank's gun burst and impact puffs (2392), and the fireball filling Paris 851's frame match the control in position, shape, shading and fade.
  - Native's smoke and dust reach below where the browser's ground and buildings cut them (Paris 1263's column, the dust under market-town's tanks), because native has no ground yet.
- **Unprimed critique** (all 24 pairs and two crops; told only that the ground and buildings are missing). It found no quad edges, banding, fringes, blending or noise errors. It said tracers, muzzle flashes, dust trails, the rocket trail, the smoke columns, the in-fireball frame, the units and the sky match. Its notes:
  - The gun burst in crop 2392 is paler and less saturated. It sits over pale haze instead of dark grass. The premultiplied blend lays the fire over the background, and that small flipbook samples a box-filtered mip (the browser's is Chrome-resized). Close up (Paris 851) the fireball is identical. *Low priority; recheck once the ground is drawn.*
  - Thin antennas read crisper and stepped against the haze. That is model geometry (step 5a) against a high-contrast background.
  - Denser-looking smoke, dust that hides a tank less, and a few extra grey puffs: each sits where the browser's ground or buildings would cover or back it.
- **Tests.**
  - `fog.rs`: a tint changes hue and never brightness; out-of-range styles are refused by name; the search reaches one pixel past the widest edge; a hard edge with no rim needs no distances.
  - `uniforms.rs`: as above. `records_are_current` passes.
  - Web: `fogStyle.test.ts` (the binding: every fixture style resolves, a bad one is refused by name, the widest edge fits the cap) and `shaderExport.test.ts` (unchanged export) pass. The typecheck is clean.
  - `cargo test` passes for `native-client` (under the lock), `presentation` and `pipelines`. Clippy `-D warnings` is clean on those three and on `presentation-wasm`.
- **Scenes.** `fog-look` passes all its checks on the branch, with every named style resolved by Rust and tuned live: the rim, seen pixels untouched, every material path styled, units never fogged, buildings fogged whole. It needs `CARGO_TARGET_DIR` set to the worktree's target for its `/__mechanics` endpoint; without it the endpoint returns 500 on the base and the branch alike.
- **Browser frame time** (`graphics-fps`, Paris-corner, production, headless, three runs per side interleaved, base web files against the branch's; `throwaway/perf/phase5e-{base,branch}/summary.md`). The look is written once, when the frame is built, so per-frame work is unchanged.

  | | avg FPS, median (min–max) | frame ms p50 / p95 | GPU ms mean |
  | --- | --- | --- | --- |
  | Branch | 94.9 (80.7–97.6) | 8.34 / 16.68 | 6.56 |
  | Base | 81.9 (76.6–87.0) | 8.40 / 17.10 | 7.46 |

  No regression. The spread is the headless pacing.
- **After merging main** (`356a0b7f4`): `cargo test` passes for `native-client` (under the lock), `presentation` and `pipelines`. Clippy `-D warnings` and `cargo fmt --check` are clean. The typecheck and the two web tests pass. Model counts equal the browser's on all 24 plates of the new control.

### Code

Net lines in this step, without the merge:

| | Rust | TypeScript |
| --- | --- | --- |
| Production | +766: native client +544 (`effects.rs` 270, `fog_mask.rs` 141, the wiring); `presentation::fog`, `FogLook` and the binding +222 | −64 (the TS fog style derivation and validation are deleted) |
| Tests | +120 (`fog.rs`'s tests, the uniforms case) | −46 (`fogStyle.test.ts` is now a binding test) |
| Generated | | +41 (`records.gen.ts`) |

Docs +9 (the native client's README).

### Decisions

- **The fog look moved to Rust, web first.** Native needs the look's uniform, and the browser derived it in TS. Now both read `FogLook`.
  - The renderer API takes the resolved words (`fogLook`, `setFogLook`), not the authored style.
  - `gameFog.ts` no longer checks the fixture's styles when it is imported. A style is checked when it is resolved. Native checks every named style (`FogPresentation::selected`).
- **Fog of war's sight is not built.** Its inputs belong to the static world, and moving `FogInput` to Rust is its own web-first step. Neither reel draws fog, so no plate could judge it yet. The fog bind group stays all zeros, which is the browser's own state in the reel.
- **The flipbook list is read from `flipbooks.ts`,** as the DFG table is read from `dfgLut.ts`, so the sheets have one owner. The mips are a box chain. The browser's are Chrome-resized, which matters only for small, distant flipbooks.
- **The tick-boundary count differences are recorded, not worked around.** The replay pushes every sample up to the record's tick, as it did for the models. Changing that rule (for example, holding back the next tick at a boundary) fixed some boundary plates and broke others. No rule tried reproduced the browser.

### Next pickup

- **Pin the capture's presentation inputs.** Record with each frame what the page's presentation framed from: the latest publication it had noted, the exact time it framed at, and the camera it was handed. Then the replay can match it on tick-boundary frames. This is a browser capture change (`reelRun.ts`, `LabViewport`, `useBattleSession`).
  - Check first whether the browser's effect counts and its drawn frame come from the same `presentation.frame` call on starved frames. `LabViewport` keeps the last `animated` results.
  - `throwaway/phase5e/replay.mjs` replays a capture through the browser's WASM module, for comparing the two quickly.
- **Fog of war's sight,** after slice 20b and step 4. Move `FogInput` into `crates/presentation` (web first), then port `fogVisibility`'s rebuild schedule and tile cull over the exported `fog*` compute pipelines. Judge it with a fog-on scene: the reel has none, so it needs a fog-look capture or a battle capture with fog.
- **Recheck small fireballs' saturation** once the ground is drawn behind them.
## Step 7: sound, built (2026-10-12, branch `phase5f`)

### The contract

- **Rust decides what is heard, once for both clients** (`crates/presentation/src/sound/`). The browser's `SoundFrame` (TypeScript) moved there whole:
  - what plays at each launch, glance, impact, blast and cook-off, the fire and motor loops, each vehicle's engine, running gear, turret and back-up beeper, footsteps by distance walked, and unseen enemies as direction-only cues (unseen shouts included: main's new `shout` category falls to the cue table's `default`);
  - when (the presentation clock, holds, late drops, a new battle), how loud and how coloured by distance (gain floor, air low-pass, reverb send, softened onset), and the voice budget with its stealing (loudest first, loops capped apart).
  - Its input is the publication as the effects take it, the frame's emitters (`Frame.emitters`, each vehicle's `class` now its unit type's index, resolved through the world input's new `vehicleClass`) and the side's hearing cues. Its output is an ordered list of `Command`s: `start` a voice (sound, variant, bus, start time, gain, rate, low-pass, wet, attack, position or pan, loop), `set` a loop's parameters, `stop` a voice with a fade, `listen` from here.
  - Its tables are `presentation.audio`, the sound catalog and the weapon rows: a derived weapon is heard as its nearest styled ancestor, now in Rust (`soundFeed.ts`'s two inheritance passes are deleted).
- **A player only carries commands out.** It tells Rust each prepared sound's length per alternative once, and the player clock (`now`) with each call.
  - The browser: `presentation-wasm`'s `SoundFrame` (`note(presentation)`, `hear(presentation, wall, now)`; `note_scripted` and `update_scripted` for the sound lab's script; the script's motion and listener go in as flat `Float64Array`s, not JSON, since parsing 200 soldiers' JSON every frame cost more than the sound itself). `packages/battle-audio/src/soundFrame.ts` is now the `VoiceSink` interface and a thin carrier of the JSON commands. `frameSound`, the heard-publication JSON (`sound_json`/`soundPublication`) and the TS listener rule are deleted; `cameraListener` calls Rust's `Listener::of_camera`.
  - Native: `native/client/src/audio.rs` on `kira` 0.12 (only its `cpal` feature). A voice is a static sound on its own track under its bus (units, effects, ambience, voices), with a low-pass filter, an equal-power pan, and a send to its bus's reverb; the master goes through a limiter. Output is the device, or a buffer filled up to each player time it is told of (tests, and the replay's `--sound <wav>`).
- **The prepared sounds** are composed by `sound::bank` (each recipe's alternatives, synthesized support under every shot of a burst, seamless mixed loops, K-weighted source calibration) from `sound::synth` (every synthesized baseline) and the clips in `assets/runtime/audio/clips`, read by a small strict 16-bit PCM WAV reader.
- **`js_number` now writes digit ties as JavaScript does** (to even: `167.43655395507812`, not Rust's `…13`). Sound keys its variations by position text; the effects' seeds read the same helper.

### What plays, and what is left

- **Plays natively, from the same data:** every sound the browser's battle plays today: gunfire with recorded bursts, impacts, ricochets, blasts, cook-offs, fires, missile motors, vehicles, footsteps, the countryside bed and hearing cues, with the same mixing decisions.
- ~~**In the browser's battle, not native yet:** unit voice lines and barks and battle music.~~ Native plays both since [the score's step](#step-7b-the-scores-decisions-in-rust-native-plays-voice-lines-and-the-score-2026-10-12-branch-music5), which moved the score's decisions into Rust.
- **One owner, both players:** the browser's bank synthesizes, composes and calibrates through the same Rust native uses (`presentation-wasm`'s `synthesize`, `RecipeBank`, `source_normalization`, `max_momentary_loudness`); the TypeScript DSP is deleted, and the workbench and the sound scene read the generated name table (`synthSounds.gen.ts`). Only the browser's convolution reverb impulse is TypeScript (`webAudioSink.ts`), a player difference below.
- **Player differences, named:** kira's reverb is a Freeverb, not the browser's convolution with its own impulse; glides are 60 ms linear tweens rather than Web Audio's 60 ms time constant; kira treats −60 dB as silence (the cull is −48 dB, so nothing audible is lost).

### Evidence

- **Equal to the browser's decisions.**
  - `crates/presentation/tests/fixtures/sound-vectors.json`: written by the TypeScript `SoundFrame` before it was deleted, over the sound lab's 8 s firefight (rifles, a tank's AP, HE and HMG, a ricochet, a missile and its motor, a burning wreck, cues), a pause, a cook-off with its turret's clang, a reversing truck and a new battle: 905 steps, about 3,800 commands. `every_command_equals_the_browsers` holds Rust to every one, floats within 1e-9 relative. Falsified once (the vehicle speed easing changed from 0.25 to 0.26 s).
  - Whole captures (scratch, `throwaway/phase5f/`): both scenes of control `2026-10-11T18-47-00-317Z` driven through the TypeScript frame and the Rust frame with the same inputs and lengths. Paris-corner's 16,284 commands and market-town's 140,069 are equal (12,480 and 29,816 byte for byte before the `js_number` fix found the one difference, a position written `…13` against `…12`). Counts: paris-corner 2,158 started, 299 dropped, 336 stolen, 12 cues; market-town 7,080, 12,147, 3,673, 278.
  - The existing browser tests run unchanged in substance on the Rust frame: `soundFrame.test.ts` (fog of war, the 100-a-side budget, holds, reset, wrecks, floor at the farthest zoom, choices, bursts, ricochets, cook-offs, helicopters, reverse), `audio`, `battleAudio`, `appAudio`; 183 audio tests across 19 files pass after merging main. Two test edits: a burst test's recipes gained the fields a catalog requires, and the distance-curve test moved to Rust.
- **Rust behaviour tests** (`tests/sound.rs`): near and far cannon (the far sound, never under the floor, darker and wetter); the transient budget holds with the loudest kept; an unseen enemy is heard only by direction; a standing clock silences transients and holds loops; every synthesized sound peaks at 0.9, is finite and loops as declared; the bank calibrates every battle sound to −23 LUFS times its authored gain; K-weighting reads −3.01 LKFS for a full-scale 997 Hz sine at 44.1 and 48 kHz.
- **The native player** (`native/client/tests/audio.rs`, rendered to a buffer): a cannon at 20 m sounds over the bed and more than twice as loud as at 1.5 km, the limiter holds, the shot plays out and frees its voice while the bed loops on; a shot to the east is louder on the right and one to the west on the left (falsified once by flipping the pan).
- **Native end to end** (scratch, both captures, buffer output, no device): paris-corner 41 s and market-town 115 s rendered; peaks 0.31 and 0.45; the frame started 2,161 and 7,078 transients. These differ from the TypeScript run by 0.1–0.2 % because a launch position in today's presentation differs from the earlier capture dump in its last bit, which changes one variant hash and its length. The player never held more than the budget plus the bed, up to one render chunk (2.7 ms) late in freeing.
- **Gates:** `cargo test` for `presentation` and `native-client` (pipelines under the GPU lock), clippy `-D warnings` on `presentation`, `presentation-wasm` and `native-client`, the web typecheck, and the audio web tests pass. `godot-binding` still compiles. `native-client`'s `tests/models.rs` needs the rifle squad's bundle textures from LFS, which this worktree does not have; it is untouched by this step.

### Code

Against the merged main: production Rust +3,838 (the sound module 2,568 code lines, of which the synthesized baselines are 833; the kira player 449; the WebAssembly binding 127); production TypeScript −684; tests +508; the vector fixture is one generated JSON line (1.2 MB). The step adds code until the browser's bank takes Rust's samples (about −750 TypeScript) and Godot's audio leaves at step 8.

### Decisions

- **Commands, not emitters, are the player contract.** `Frame.emitters` stays the sound's input; a player never reads it. Both players are as dumb as a mixer can be: placement by direction (the Web Audio panner's equal-power law) is the only geometry they do.
- **Rust is given the prepared lengths** rather than composing the browser's buffers itself, so the browser's bank stays the browser's until it moves.
- **Native renders offline from the replay** (`--sound <wav>`), never to the device, as the brief required. A live client opens `OutputSettings::Device`.
- **The sound vectors are read with exact float parsing** (`serde_json`'s `float_roundtrip`, as a dev-dependency of `presentation` only): variations are keyed by position text, so a one-ulp parse error changed a variant.

## Step 8: Godot deleted (2026-10-12, branch `phase5h`)

The deletion half of step 8. The gates half (plates, counts and performance against the browser, release build) is still open.

### The contract

- **Gone:** `native/godot` (the Godot project: its GDScript, Godot shaders, parity runner, release export, bench and tests) and `native/godot-binding` (the GDExtension), with their workspace entries and the `godot` crate tree in `Cargo.lock`, and `crates/sim/examples/native_binding_probe.rs`, the binding's offline control.
- **Two presentation calls only the binding made are removed:** `static_world::files::buildings` (a resolved `Buildings` for a renderer that walks them itself; native reads `files::placed_buildings` through `load_world`) and `View::for_detail` (a matrix-free view for Godot's own camera). Nothing on `Presentation` or either WASM module was Godot-only.
- **The parity harness is the browser capture alone.** `web/scenes/native-parity-capture.mjs` is unchanged; Godot's runner was `parity_run.gd` and went with the project. `fps-compare.mjs` no longer reads Godot's render report, so it compares browser runs until `native/client` writes a bench. `graphics-fps` counts a running `native-client` as GPU contention. `gpu-trace.mjs` and its test name `native-client`.
- **The browser capture fixtures the Rust tests read moved** to `fixtures/parity/capture/` (`run/paris-corner/`: capture, manifest and clip table; `poses/paris-corner-poses.json`; `publication/`: the market-town capture and both scenes' browser decodes). `sim`'s reader test, `presentation`'s pose and effect tests and `native-client`'s corpse test read them there. The Godot-only fixtures nothing else read are deleted: the height, grass-sampling, learned-ground and water vectors, Godot's own effect vectors and units section, its map copy, and the run's terrain and grass artifacts.
- **Kept, and why:** `packages/scene-assets/blender/packs.{json,py}` name paths inside a third-party asset pack (`Unreal-Godot/`, `Godot - UE/`); they are the vendor's folder names. The spec's earlier slices, `choices.md` and `specs/done/` record what was built on Godot.

### Evidence

- **The harness still feeds native.** A fresh paris-corner control from this branch (`throwaway/native-parity/2026-10-12T13-44-39-698Z`) passes `native-parity-capture`. `native-client` replays it, under the lock, into six plates (`throwaway/phase5h/plates/`). Model and building-module counts equal the control's on all six. Effects are equal on four. The other two, 1195 and 1593, are drawn exactly on tick boundaries (sim ticks 959 and 1169), which is step 6's known capture rule.
- **Tests:** `cargo build --workspace`; `cargo test` for `native-client` (all five targets, under the lock), and for `presentation`'s lib, `effects`, `poses` and `pose_behaviour`, and `sim`'s publication reader, all reading the moved fixtures; clippy `-D warnings` on `presentation`, `sim`, `native-client` and `presentation-wasm`; `cargo fmt --check`; the web typecheck; `gpuTrace.test.ts` and `frameComparison.test.ts`.

### Code

Net lines deleted, by language (generated `Cargo.lock` −129 apart):

| | Deleted |
| --- | --- |
| GDScript | 11,264 |
| Godot shaders (`.gdshader`, `.gdshaderinc`, the pose `.compute`) | 2,105 |
| Rust (the binding, the probe and the two calls; +8 −1,340) | 1,332 |
| Markdown (Godot's and the binding's READMEs, docs) | 560 |
| Shell, Python, and the Godot project, scene and export files | 201 |
| TOML (the binding's manifest, the workspace) | 16 |
| JavaScript and TypeScript (`fps-compare`'s Godot reader; the trace test grew in formatting) | 3 |

Fixtures: about 0.6 MB of Godot-only vectors and run artifacts deleted.

### The performance gate's measure (2026-10-12, branch `nativebench`)

The gate is "native at or below the browser on every measure, both scenes, release build". This builds native's half of the measure and takes one provisional reading. The final gate is rerun once native draws everything.

**The contract.**
- `native-client --bench <dir>` (`native/client/src/bench.rs`) plays a parity capture's frame records back to back. Each frame is framed by `Presentation::frame` at the record's sim time and camera, with every publication up to its tick, then drawn through the whole pass list and presented to the borderless off-screen window. No plates are written.
- Each frame records three times:
  - its interval, from one frame's start to the next;
  - its CPU time, from the start through the submit and the present call;
  - its GPU time between two timestamp marker passes. These are `gpuTiming.ts`'s 1×1 cleared render passes: the begin marker goes in its own command buffer, submitted first, and the end marker goes at the end of the frame's encoder.
- The CPU time is also split into framing, preparing the layers, encoding, and submit with present.
- The off-screen window never blocks, so the bench keeps at most two frames in flight (the swap chain's latency). A frame's interval is then as long as the slower of its CPU and its GPU.
- The first 60 frames are left out, as `graphics-fps` leaves out its first 60.
- The report is `native-<scene>-<time>.json` in the browser file's shape. It has `client: "native"`, the conditions (power, build, adapter, pace, the pass list and the mean drawn per frame), and per scene the `intervalsMs`, `cpuMs`, `gpu {frames, meanMs, p95Ms}` and `phasesMs`.
- `fps-compare.mjs` reads `native-*.json` beside `browser-*.json`. Its table gains the CPU ms p95.
- **Every bench frame lists every static batch** (`Replay::frame(.., drawn = true)`), because the models and scenery hold no batch from the frame before. Native therefore copies and uploads every static instance each frame: 120 KB a frame on paris-corner and 259 KB on market-town. The browser keeps them. (Step 5c since holds them, and a bench frame writes only what changed.)

**The final reading** (2026-10-13, branch `finalbench`, at `ef8f2d2fe`). Both sides draw everything: the ground, the grass, the buildings, the scenery, the props, the models, the effects, fog's mask (no fog of war: the reel has none) and post.
- **Method:** a fresh control (`throwaway/native-parity/2026-10-13T15-35-25-248Z`, both scenes, passes `native-parity-capture`), which native replays. Native is `--bench` on a release build; the browser is `graphics-fps` on a production build. Runs were interleaved native and browser, plugged in, Apple M5 Pro, 1280×720. Each timed run held the GPU lock and started at a 1-minute load under 4: native 2.4–3.8, the browser 2.3–3.9. A duplicate runner made it four runs a side, not three; every run met the same conditions, so all four count. The rows are `fps-compare.mjs`'s medians (`throwaway/perf/finalbench/summary.md`).
- **Only the browser** also runs the live simulation worker, the HUD and the sound frame, and draws its own frames from the live simulation. **Native** replays the capture's 8,461 and 4,001 records (8,400 and 3,940 timed).
- **Native's mean per frame:**
  - market-town: 42,879 scenery instances, 4,802 building pieces, 17,094 props, and models with 2.85M triangles (12 soldiers, 4 vehicles, 10 corpses), plus 509 effects;
  - paris-corner: 556 scenery instances, 15,038 building pieces, 21 props, and models with 3.1M triangles (16 soldiers, 1 corpse), plus 60 effects.

| client | scene | avg FPS (min–max of runs) | 1% low | min FPS | frame ms p50 / p95 / p99 / max | CPU ms p50 / p95 | GPU ms mean |
| --- | --- | --- | --- | --- | --- | --- | --- |
| browser | market-town | 77.9 (77.7–78.0) | 39.6 | 37.1 | 15.01 / **17.74** / 25.00 / 26.97 | 2.32 / **3.37** | **8.60** |
| native | market-town | 78.3 (67.7–78.3) | 51.7 | 42.4 | 13.00 / 18.05 / 18.95 / 23.59 | 2.30 / 4.27 | 12.15 |
| browser | paris-corner | 105.7 (105.5–105.7) | 56.6 | 40.0 | 8.34 / 16.67 / 16.85 / 24.98 | **1.20** / 1.54 | **5.35** |
| native | paris-corner | 112.8 (112.7–112.8) | 73.2 | 68.4 | 8.35 / 12.75 / 13.34 / 14.61 | 1.30 / 1.44 | 8.46 |

Bold marks where the browser is lower.

**Verdict: the gate fails.** Native leads on average FPS, 1% low, minimum FPS, and frame p99 and max on both scenes. It misses on five measures:
- **GPU mean, both scenes:** 12.15 against 8.60 ms on market-town and 8.46 against 5.35 on paris-corner, about 40–60% more GPU time per frame. The runs agree closely: native 12.09–14.24 and 8.46–8.49, the browser 8.56–8.60 and 5.27–5.38.
- **Market-town's CPU p95:** 4.27 against 3.37 ms.
- **Paris-corner's CPU p50:** 1.30 against 1.20 ms.
- **Market-town's frame p95:** 18.05 against 17.74 ms, within the spread of runs.
- Paris-corner's frame p50 ties (8.35 against 8.34): the browser is held at the display's 120 Hz there, and native is GPU-bound at its own 8.4 ms.

**Where native's time goes:**
- **The GPU bounds every native frame.** The frame interval p50 matches the GPU mean: 13.0 against 12.15 ms on market-town and 8.35 against 8.46 on paris-corner. With frames serialised (none in flight while the next is built), paris-corner still takes 7.7 ms of GPU, so the gap is work and not two frames overlapping in the marker span.
- **Its CPU frame** (medians, p95 in brackets):
  - market-town: framing 0.95, preparing 0.62 (1.87), encoding 0.12, submit with present 0.73. Preparing's tail is the static batches rewritten as the camera walks; the frame lists about 2 MB of instances.
  - paris-corner: framing 0.44, preparing 0.10, encoding 0.09, submit with present 0.66.
- **Presenting costs nothing,** because the off-screen window is occluded.
- **The measures are not quite alike.** The browser's CPU time is its main thread only; Dawn's command encoding runs in Chrome's GPU process and is not counted. Native's submit is its whole `wgpu` submission. The browser's GPU mean is taken over its rolling 240-frame windows (57 and 20 readings). Native's is per frame, read back for 81–84% of them.
- **The GPU is not split by pass.** A pass ablation (each pass skipped beside a baseline, twice) was swamped: other lanes were using the GPU while the load stayed under 4, so the same baseline read 7.7–14 ms. A Metal System Trace of native assigns no frames, because the occluded window never presents.

**No fix was made.** The one cheap lead was naga's bounds checks: shaders built unchecked (`create_shader_module_trusted`) took 0.5–1.5 ms less on paris-corner in 5 of 6 paired runs, on the same contended GPU. That is unsafe, and it does not close a 3 ms gap.

The `graphics-fps` runs above completed their timing, but each failed the scene on a page error: a local production build loaded Vercel Analytics' script, which only Vercel serves (a 404). Analytics now mounts only on a Vercel build (`__ON_VERCEL__`, from Vercel's `VERCEL` build variable).

The GPU reading above is superseded: the two clients' timers measured different spans of the frame (below).

### Where native's extra GPU time went (2026-10-13, branch `nativegpu`)

**The contract:**
- **The GPU time is the frame's whole span on both clients:** from the first marker pass's start to the last marker pass's end (`gpuTiming.ts`'s `endOfPassWriteIndex`, the bench's `end_of_pass_write_index`). It was stamped at the last marker's start.
- **A scar region's scissor has one owner,** `presentation::scars::scissor`: the pixels a region's box can cover on screen, or none when it is off screen or behind the eye. The browser calls it through `presentation-wasm`'s `scar_scissor`; the TypeScript copy is gone. Native now skips an off-screen region's upload and draw, and scissors the rest, as the browser does.
- **Native's world pass resolves and discards** its multisampled colour and fog mask, as the browser's does. Only a ground region's pass keeps them for the next region.

**Method: work, not time.** A fresh control (`throwaway/native-parity/2026-10-13T20-52-26-693Z`) was captured with an init script wrapping WebGPU (`throwaway/gpuwork/browser-work.js`). For each plate frame (the frame read back from the canvas) it records every pass's attachments and load and store operations, and every pipeline's draws, vertices, instances and dispatches. Native recorded the same per plate, from its pass list, with a throwaway counter. All 24 plates compare draw for draw:
- **Shadows:** 4 cascades of 2048², the same caster draws in each (paris-corner 1259: `modelCaster` 50 draws and 2.79M vertices a cascade on both).
- **Depth prepass, world pass and bloom:** the same draws, the same targets (4× MSAA full size; bloom from half size, 5 levels), and the same pipeline state from the export.
- **Grass, fog and post:** the same grass dispatch (3,600 groups on paris-corner). No fog compute runs, since the reel has no fog of war. The same 12 bloom passes and the look.
- **No scar regions** on any plate: every mark fits at once in this reel.
- **Native drew nothing the browser did not,** except two things. First, its world pass stored both multisampled targets where the browser discards them (about 44 MB of writes a frame at 1280×720). Second, it drew impostor-tier bodies as meshes where the browser draws cards, since cards are not baked natively: at most 60k extra vertices a frame of 17M.
- **The browser draws more:** the ground paint, the x-ray, the overlays and glow, and two depth copies a frame (37 passes against native's 26).

**The other candidates, checked and equal:** shadow casters (culled per view in Rust for both), MSAA and target sizes, the early depth tests (the exported compares), static batches (drawn once), per-frame resource creation (native makes none outside a regional frame), and `wgpu`'s release flags (only indirect-draw validation, which Dawn also does; 0.004 ms a frame in the trace).

**The shader settings, compared against Dawn's own Metal Shading Language** (dumped with `dump_shaders`):
- Tint bounds loops and clamps indices as naga does, so naga's runtime checks are the browser's work too. Building them out would do less than the browser, and they stay.
- Both mark the position invariant, and both pull vertices in the shader.
- naga differs in two places: it drops the explicit level on `textureSampleCompareLevel`, and it reads vertex attributes byte by byte. Native A/B builds of each, with a patched naga and a patched `wgpu-hal`, moved the world pass within the noise. Neither is extra work.

**Where the time went: the measure.** A Metal System Trace of each client puts each pass's vertex and fragment stages on a timeline:
- In the browser, the `frame-end` marker's vertex stage started at 9.7 ms of a frame whose fragment work ran to 23.6 ms (traced, so slowed). Dawn holds a pass back only at the stage that reads, so every later pass's vertex stage ran ahead while the world was still shading.
- `wgpu` leaves Metal's per-resource hazard tracking on, which holds the whole pass back. Native's post chain and markers waited for the world's fragment work, so its end stamp fell at the frame's true end.
- Both timers stamped the end at the last marker's start, so the browser's GPU time stopped early. Taken to the end of the last marker pass (fragment work runs in order), the browser read 12.17 and 9.96 ms on market-town and paris-corner in its next run, against 8.60 and 5.35 before.

**The reading** (both clients at `bb003bf16`, control `throwaway/native-parity/2026-10-13T22-01-55-089Z`, both scenes passing `native-parity-capture` with native's counts equal on every plate. Three interleaved pairs, each run started under the GPU lock at a 1-minute load under 4 (1.7–3.9). The rows are `fps-compare.mjs`'s medians; `throwaway/perf/nativegpu/summary.md`):

| client | scene | avg FPS (min–max of runs) | 1% low | min FPS | frame ms p50 / p95 / p99 / max | CPU ms p50 / p95 | GPU ms mean (runs) |
| --- | --- | --- | --- | --- | --- | --- | --- |
| browser | market-town | 83.2 (82.3–83.4) | **39.7** | **24.0** | **9.19** / **17.00** / **23.41** / **41.59** | **2.28** / **3.27** | **10.87** (10.87, 11.04, 10.87) |
| native | market-town | 80.1 (66.6–83.8) | 32.8 | 4.1 | 12.69 / 17.47 / 24.94 / 245.16 | 2.29 / 4.07 | 11.68 (14.03, 11.68, 11.42) |
| browser | paris-corner | 108.0 (108.0–111.5) | 57.9 | 40.0 | 8.33 / 16.67 / 16.74 / 24.99 | 1.39 / 1.69 | **7.54** (7.54, 7.65, 7.17) |
| native | paris-corner | 121.0 (114.3–125.7) | 72.6 | 70.7 | 7.64 / 12.40 / 13.58 / 14.15 | 1.14 / 1.67 | 7.94 (8.41, 7.94, 7.64) |

Bold marks where the browser is lower or better.

**Verdict: the gate still fails, now narrowly.** Measured to the frame's end on both clients, native's GPU frame is 5–7% longer, where the old reading put it 40–60% longer:
- market-town 11.68 against 10.87 ms;
- paris-corner 7.94 against 7.54 ms;
- in the quietest pair (load 1.7–2.8), 11.42 against 10.87 and 7.64 against 7.17.

Native leads on every paris-corner measure but the GPU mean. On market-town the browser leads on most measures. Native's first market-town run was contended: every CPU phase doubled and the GPU read 14.03. Its tail (min FPS 4.1, max 245 ms) is a single stall, inside the first run.

**What is left, and where:** per-pass timestamps over the paris-corner reel put native's remaining excess in the world colour pass (6.2 against 5.6 ms from vertex start to fragment end), with the same draws and pipeline state. The cascades are within 0.1–0.2 ms, and the depth prepass is faster on native. The two shader-level differences found (naga's implicit-level compare lookups and byte-wise vertex fetch) did not move it.

The remaining lead is `wgpu`'s whole-pass hazard tracking. Dawn lets later passes' vertex work, and the next frame's cascades, overlap the world's shading; `wgpu`'s tracked Metal resources serialise them. It is untested here.

**Next for the gate:** test that lead. One way is a `wgpu` build whose Metal resources are untracked (`wgpu-hal` has the `HazardTrackingModeUntracked` TODOs), A/B'd on the frame's span. The other is a native frame that lets the cascades of frame N+1 start before frame N's world pass ends. Either is a dependency or architecture decision for the user, not a setting.

Native's market-town plates also differ run to run (the same binary, 0 of 18 plates byte-identical between two replays, under 0.3% of pixels). Paris-corner's are exact, so the likely source is the grass build's append order. This predates this branch.

### The gap's last experiment, and the gate closed (2026-10-14, branch `passoverlap`)

**The question:** does any `wgpu` setting, short of restructuring native's frame, let its passes overlap as Dawn's do?

**The candidates, read in `wgpu` 30.0.1's source before any were measured.** Only one could move the GPU frame:
- **Hazard tracking.** Metal tracks every buffer and texture `wgpu-hal` creates; untracked is a `TODO` in `create_buffer` and `create_texture`, with no option to reach it. Turning it off means patching `wgpu-hal` and fencing by hand, which is not a setting.
- **Backend options.** `BackendOptions` has GL, DX12 and noop fields and none for Metal. Metal's `open` ignores `MemoryHints`.
- **Instance flags.** A release build's default is `VALIDATION_INDIRECT_CALL` alone, with no debug flags and no validation. That flag injects a compute pass checking the grass's indirect draws in every frame. **It was the one candidate measured.**
- **Present mode and `desired_maximum_frame_latency`.** The bench's window is off-screen and presents nothing, and the bench keeps its own `IN_FLIGHT` of two. Neither setting is on the timed path.
- **Usages.** Every render target is `RENDER_ATTACHMENT`, plus `TEXTURE_BINDING` where something samples it. Only the display adds `COPY_SRC`. Nothing carries `STORAGE_BINDING`, so no target gets Metal's `ShaderWrite`. The depth format is `Depth32Float`, which does not get `PixelFormatView`. Only the bench's and the plates' readback buffers are mappable, so every other buffer is private.

**The measure.** A fresh control (`throwaway/native-parity/2026-10-14T00-44-09-571Z`, both scenes, passes `native-parity-capture`). One release binary was run with the default flags and with `InstanceFlags::empty()`, in three interleaved pairs a scene. Each run started under the GPU lock at a 1-minute load under 4 (1.71–3.94). GPU ms mean (`throwaway/passoverlap/noindirect/`):

| scene | default flags (runs) | no indirect validation (runs) | median change |
| --- | --- | --- | --- |
| market-town | 11.78, 11.53, 11.47 | 12.25, 11.42, 11.45 | 11.53 → 11.45 |
| paris-corner | 8.04, 7.77, 7.67 | 7.84, 7.78, 7.67 | 7.77 → 7.78 |

Within the spread of runs: the validation pass costs nothing measurable, and the change was reverted. The default flags' medians reproduce `nativegpu`'s reading (11.68 and 7.94 ms). No Metal System Trace was taken, since there was no overlap change to show.

**Verdict: the gate is closed, with one named exception.** Native's GPU frame stays 5–7% longer than the browser's. That is the cost of `wgpu`'s tracked Metal resources, and it is accepted (`choices.md`, "Native's GPU frame is 5–7% longer than the browser's").

What the player gets is at or above the browser's. This branch's default-flag runs (`fps-compare.mjs`'s medians) are set against `nativegpu`'s browser rows, which were not rerun:

| scene | avg FPS (native / browser) | 1% low | frame p99 ms |
| --- | --- | --- | --- |
| market-town | 83.2 / 83.2 | 56.5 / 39.7 | 17.38 / 23.41 |
| paris-corner | 123.2 / 108.0 | 81.3 / 57.9 | 11.24 / 16.74 |

Native's paris-corner min FPS (30.9, against 40.0) is a single stalled frame in one run. Counts were equal on every plate of both scenes, replayed from the merged main (`91ec9e8e3`).

## Step 5b: the scenery, and the ground on `Presentation` (2026-10-12, branch `native5i`)

Native draws the scenery from `Frame`'s static batches (slice 20b step 4's walk) and reads its ground from `Presentation`. The map's props and the known props are the `props20b` lane's, not yet on main.

### The contract

- **`Presentation::place_scenery(finest: &[&[f32]])`** (amends 20b step 4) takes each scenery kind's finest tier in the renderer's world vertex (`ground::mesh::VERTEX_FLOATS`, 1 in the last float for foliage), in `scenery_kinds` order, and refuses the wrong count. Rust reads off it what the walk needs: the kind's height and reach, and a tree's trunk radius at the stump's top and its crown base (`static_world::ground::kinds`). A tree is a kind the biome names in `trees.species`.
  - It was four numbers a kind, computed by the page (`sceneryFacts`).
  - WASM: `place_scenery(meshes: Float32Array, floats: Uint32Array)`, the meshes one after another.
- **`Presentation::stump_mesh(row) -> Option<&[f32]>`** is a felled tree's stump row's mesh (`Appearance.draws` 6): its kind's trunk cut at the biome's `stump_height_m`, in its bark's mean colour, the top in `felled.cut`. None for any other row, and for a shrub's or dressing's stump row, which nothing draws. WASM: `stump_mesh(row)`; the page puts it on the row (`PresentedAppearance.stump`, `WorldAppearance.stump`), and the scenery layer draws it.
  - Deleted from TS: `sceneryFacts`, `kindSize`, `crownBase`, `trunkAt`, `stumpMesh` (`scenery/appearance.ts`), `WorldScenery.felled`, and `felled.test.ts` (its case moved to `kinds.rs`).
- **The page places the scenery from the world's scenery art**, not from the art its buildings draw: `SimPresentation.setGround(ground, scenery: WorldScenery | null)`, which `LabViewport` passes from the world layers. Since 20b step 4 the battle had handed its presentation the models' pruned art (`mapAppearances`), which holds no scenery, so **no battle and no graphics test drew any scenery**. `native-parity-capture` now checks that every scene draws some.
- **Native sets its ground on `Presentation`.**
  - `files::ground(root, map)` returns what `set_ground` resolves from (`Source`, `Biome`, `Tuning`) and the simulation's terrain triangles, and no longer builds a `Ground`.
  - `Replay::open(dir, game, library)` sets the ground, places the scenery from the bundles, then loads the world.
  - `Ground::new` and `Grass::new` read `Presentation::ground()`; `Passes::prepare` takes the presentation.
  - `Presentation::set_scar_reach` is deleted: `set_ground` sets the reach for both clients.
  - The replay hands back the ground the side has seen cleared after each frame (`set_cleared`), as `LabViewport` does.
- **Native's scenery layer** (`native/client/src/scenery.rs`) holds each scenery row and each kind's four tiers, packed as `tierMesh` packs them (shared by every row of the kind). It draws each batch from the frame's instance buffer at its first record. Routing by `draws` is `sceneryLayer.ts`'s:
  - prepass: `felledPrepass`, `dressingPrepass`, otherwise `sceneryPrepass`;
  - casters: `felledCaster`, otherwise `sceneryCaster`;
  - lit: `backdropColour`, `felledColour`, `dressingColour`, otherwise `forestColour`.
  
  A felled tree's two records make one instance. Scenery draws after the ground in the prepass, after the models' casters in each cascade, and after the buildings in the lit pass, as `worldPass.ts` orders them. The model layer leaves rows with `draws ≠ 0` to it.
- **The replay checks the scenery counts** (`instanceCounts[plate].scenery`: per population, kind and tier, and stumps per kind) beside the model and effect counts.

### What draws, and what is left

- **Draws:** the forest, the tree lines' shrubs, the scenery past the map, the forest floor's dressing (shrinking as it nears a few pixels), and felled trees with their stumps (no plate in either reel has a felled tree). All of it is shadowed and cast into the cascades as the walk lists it.
- **Left:** the map's props and the known props (parked cars, lamps, bollards, fences, walls, hedges, benches, resting wrecks). They belong to `props20b`, which had not landed on main when this step closed. Painted marks and fog of war's sight are as in step 4.

### Evidence

Control `throwaway/native-parity/2026-10-12T17-33-32-749Z`, captured from this branch's tree. Native plates, pairs and crops are in `throwaway/native5i/` (`n1/`, `cmp/out/`, `cmp/crops/`).

- **Equality, before the TypeScript was deleted.** A Vitest dump of the TS (V8, run on every tree, hedgerow and dressing appearance in the catalog: 13 kinds) against Rust:
  - all four facts are equal bit for bit, and all 7 trees' stumps are equal float for float;
  - native's `tier_mesh` equals the browser's `tierMesh` float for float on every kind's finest tier (up to 279,360 floats).
  
  Under Bun (JavaScriptCore), one fact differed by an ulp: JSC's `Math.hypot` is not V8's, which `js::hypot` reproduces.
- **Counts:**
  - Scenery counts (every population, kind and tier, and stumps) and model counts are equal on all 24 plates.
  - Effects are equal on every plate drawn between ticks. The 7 market-town plates drawn exactly on a tick boundary differ, as in step 6.
  - Paris-corner draws only backdrop scenery (501–628 a plate); market-town draws up to 25,659 forest, 347 shrubs, 30,208 backdrop and 2,152 dressing instances.
- **Plates** (`compare-screenshots`, all 24):

  | | grayscale MAE | pixelmatch ratio |
  | --- | --- | --- |
  | market-town | 0.05–0.56 (step 4: 0.72–3.48) | 0.00004–0.011 |
  | paris-corner | 0–0.74, and 4.54 on 169 | 0–0.015, and 0.074 on 169 |

  Paris 169 is the street of parked cars, all of them props.
- **Unprimed critique** (all 24 pairs and four 2× crops, told only that they are two renderers of one moment): trees, copses and forest match in position, size, colour and shadows, and so do the shrubs, the clutter under the trees, the craters, the fields, the grass, the buildings, the effects and the units. Every difference it found is street furniture absent in native: cars, fences, walls, hedges, benches, tables, lamps, bollards, utility boxes and the static wreck hulks. It found no rendering defect in either. Its low-confidence notes (a tank's road-wheel shading, a pixel at distant soldiers, faint grass speckle) are within native's run-to-run floor (step 4).
- **The browser regression:** the base (`003002e0c`) capture of market-town drew 0 scenery on all 18 plates, and its plates have no trees. The branch's capture draws them.
- **Tests:**
  - `cargo test` passes for `presentation`, `pipelines` and `native-client` (under the lock).
  - `kinds.rs` holds three cases, and `native/client/tests/scenery.rs` checks that every tree kind's stump row is its trunk cut at the biome's height. That test was falsified once (stump cut 10 % high).
  - Clippy `-D warnings` and `cargo fmt --check` are clean.
  - The web typecheck and oxlint are clean, and 8 affected web test files pass (48 tests).
  - `native-parity-capture` passes on both scenes.

### Code

Net lines, comments and blanks excluded:

| | Rust | TypeScript and JS |
| --- | --- | --- |
| Production | +533 −57 in-module tests = **+476**: native `scenery.rs` 240, `ground/kinds.rs` 119, the rest the wiring (the replay's ground and scenery, the passes, the counts) | **−75** (TS +43 −119, TSX +1) |
| Tests | +109 (`kinds.rs` cases, `tests/scenery.rs`) | −29 (`felled.test.ts`); the capture scene's check +5 |

Generated: `records.gen.ts` (one comment). Docs: the native client's and `presentation-wasm`'s READMEs.

### Decisions

- **The scenery facts and the stump moved to Rust, browser first**, rather than being ported a second time into the native client. Each renderer still packs a bundle's tier into its own vertex (`tierMesh`, `tier_mesh`), as each packs models.
- **A tree is a biome species** in Rust. The page used the catalog's `scenery: "tree"`; the two agree on every kind.
- **The page's scenery art is the world layers' `scenery`** (the biome's appearances), handed with the ground. It is no longer whatever `setBuildings` was given.
- **Native shares a kind's tiers across its rows.** The browser uploads them once per row: up to five copies of each kind.
- **Native draws the scenery from the frame's instance buffer.** It keeps no buffer per batch key. That is right for the replay, which resends the statics on every drawn frame. A live client needs per-key buffers, as the buildings do (step 4's decision).
- **The cleared ground is still handed back by the renderer** (20b step 4's decision; the lab can hide it). Native's replay does what the page does. Folding it into `Presentation` with a "shown" switch would remove that round trip from both clients.

### Next pickup

- Merge `props20b` when it lands and draw the props' batches. The model layer already loads any static row, and the critique's differences are all props.
- Then step 8's gates: plates, counts and performance on both scenes in a release build.

## Step 7b: the score's decisions in Rust; native plays voice lines and the score (2026-10-12, branch `music5`)

Settles battle voices and music's choice N2 (see `choices.md`).

### The contract

- **Rust decides the battle score** (`presentation::sound::music::Score`), as it decides every other sound. In: the barks the line arbitration heard (`Lines.heard`) and whether a line sounds (`Chatter::speaking`, new). Out: music commands. It owns the mood (`tension`, `combat`, with the enter and leave spans of `presentation.audio.music`), the next track of a mood as a logical catalog id (a shuffle bag on its own seeded `Mulberry32`), and the dip under lines. `presentation.audio.music` and the catalog's `music.moods` are typed in `sound::tables` (they were an unread `Value`).
- **`Command` gains three music ops** in the same stream as `start`, `set`, `stop` and `listen`:
  - `{op: "music", at, track, fade}`: stream `track` on the music bus, looping, fading in over `fade`;
  - `{op: "music_stop", at, fade}`: fade every sounding track out over `fade`, then free it;
  - `{op: "duck", at, depth, glide}`: dip the bus by `depth` (0 restores), gliding with time constant `glide`.

  A mood change is `music_stop` then `music` over `crossfade_s` (a crossfade); the first track only fades in.
- **A player resolves a track id to its file.** Rust never sees a name, format or path. Menu music stays the app's.
- **Browser:** `presentation-wasm` binds `Score` (`new(rules, moods, seed)`, `hear(now, barks, speaking)`) and `Chatter.speaking`. `battleMusic.ts` only carries the commands out on `MusicPlayer`, whose `duck(depth, glide)` lost its attack/release choice. `musicDirector.ts` and its test are deleted; the workbench's Music tab and the sound lab's barks drive the Rust score.
- **Native** (`native/client/src/audio/`): `Audio::speak(now)` (run by `hear`) arbitrates the lines and the score after the sound. `Audio::speakers` loads every named set's `pack.json`, and `Audio::respond` asks for an answer. The voices bus plays a line's take, read and decoded when it plays, with the browser's calibration. The music bus (level, then a duck) streams each track's MP3 packet by packet with amplitude fades. A missing pack, take or track is refused by name (`Audio::errors`, printed by the replay).

### Evidence

- **Equal to the TypeScript score:** `crates/presentation/tests/fixtures/music-vectors.json`, recorded from `BattleMusic` over `MusicDirector` before they were deleted, its chance drawn from `mulberry32` by seed. Eight cases: five seeds over a 20-minute battle script (fights coming and going; enemy barks, own non-fighting barks and answers that must not count; lines sounding), the calm span's edges with every combat trigger, a mood with no tracks, and two tracks a mood. That is 16,824 steps with 1,712 calls: 8 first tracks, 169 crossfades and 1,535 dips. `every_decision_equals_the_recorded_vectors` holds every command at every step. Falsified once: dropping the never-twice swap fails it, and the shuffle test with it.
- **Rust behaviour** (`tests/music.rs`, `tests/voices.rs`): only an own fighting bark brings combat, and the calm span's edge is held exactly; a change crossfades and the first track only fades in; over 50 seeds and 30 fights each, every track plays once a shuffle and never twice in a row; the seed decides the order; a mood without tracks starts nothing; the dip and its recovery; a line is heard speaking until it ends, through a reset.
- **Browser:** every web test touching battle audio passes (24 files, 193 tests), `battleMusic.test.ts` now holding the carrier to Rust's commands. One test's hand-built catalog gained the `music` section the catalog type requires, which Rust now reads.
- **Native, rendered to a buffer** (`tests/audio.rs`, needs one track and one pack's bark from LFS): four ten-second runs separate the score, the line and the dip by subtraction. The track fades in from silence (RMS 0.0003 in the first 0.3 s, against 0.038 once in) and streams on. A seen enemy tank's "under fire" bark plays its 1.92 s take (RMS 0.0073 where the runs differ, nothing before it). Under it the music sits at 0.650 of its level (duck depth 0.35), and it is back at 1.000 after. Falsified twice: a no-op duck (1.0 under the line) and dropping the line's commands.
- **The `sound` scene** passes all 15 checks under the GPU lock, the score's included (tension, then combat on the own bark and not the enemy's, dipping under each line). The sound frame's cost at 100 a side is a p50 of 0.175 ms and a p95 of 0.295 ms under the lock, against a 2 ms budget. Rerun unlocked after merging main (the bank's move to Rust included), all 15 pass at a p95 of 0.580 ms. Its answer checks need `lead-selected-*` and `lead-refused-*` from LFS.
- **Gates:** `cargo test` for `presentation` (all targets) and `native-client --test audio`; clippy `-D warnings` on `presentation`, `presentation-wasm` and `native-client`; `cargo fmt`; the web typecheck; oxlint (no errors); oxfmt.

### Code

Net lines against main, by language:

| | Net |
| --- | --- |
| Rust, production | +807: the score 176 and its tables and binding 66 (`presentation`, `presentation-wasm`); native's player +562 (MP3 decoding 102, the music bus 225, voice packs 128, wiring 107) |
| Rust, tests | +429 |
| TypeScript, production | −11 (the director's 62 lines deleted; the carrier and its command types) |
| TypeScript, tests | −71 |
| Markdown | +62 |
| `Cargo.lock` and TOML | +73 (`symphonia` 0.6's MP3 decoder, already `kira`'s version) |

The vector fixture is one generated JSON line (0.6 MB).

### Decisions

In `choices.md`, "the score's decisions in Rust". Not ported: native's unseen-enemy shout pool. Its take selection is still the browser's `shoutPool.ts`, so an unseen enemy's shout is silent natively.

## Step 6b: fog of war's sight in Rust; native computes and draws it (2026-10-12, branch `fogsight`)

Step 6 left fog's sight in TypeScript: `fogInputs.ts` and the CPU half of `fogVisibility.ts` assembled the compute's input from the static world and the observation, and native bound an all-zero fog group. Browser first, that derivation moved into `crates/presentation`, equal to the TypeScript word for word, and the TypeScript is deleted. Native then runs the browser's exported fog compute from the same records.

### The contract

- **Rust decides every fog upload** (`presentation::fog::sight`). Its inputs are the ground's `Source` (the height grid, the props with the layout's `occludingPropKinds`, and now the playable rectangle), `presentation.fog_geometry` (now part of `Tuning`, checked by Rust), the rules' sensor numbers, and the side's publications. From these it derives:
  - the eyes: every own unit's `sight.eyes` at the published tick, keyed `<unit>:<eye>`;
  - the occluders the side knows: the map's occluding props less those a known prop replaced, plus known props that occlude and are not destroyed;
  - the map slots, the rebuild schedule, the horizon records, the structures' `wholes` grid and their pass rows, and the `FogParams` record.

  Each frame, `Presentation::prepare_fog(camera, width, height, probes)` returns one `FogUpload`: params, eyes, map sizes, rebuild records and workgroups, foliage, occluders and the `wholes` words when they changed, the structures pass, and the cull's tiles. A renderer writes each piece as given and runs the passes it names.
- **New records:** `FogParams` (256 B), `FogEye` (48 B) and `FogBox` (32 B). `crates/pipelines/tests/uniforms.rs` holds them to `fogMerge.wgsl`'s and `worldTerrain.wgsl`'s structs. `records.gen.ts` also generates `FOLIAGE_STEP`, `WHOLE_TEXTURE_WIDTH` and `WHOLE_BOX_WORDS`, which the shader text splices.
- **What a scene asks** (`Presentation::set_fog(FogSetup)`, or none for no fog): `sensors`, and a lab's stand-in `eyes` or the `units` it looks through. The page's `FogRequest` carries them; `ObservationDecoder.setFog` applies it once the ground is set, and `LabViewport` hands it each frame. The side's **foliage** is still the simulation's export (`WorldView.foliage_cleared`, `files::side_foliage` natively), handed over when the side has seen ground cleared.
- **The renderer** gets fog from the presented ground (`PresentedGround` includes `prepare_fog`, `resend_fog`, `rebuild_fog`, `fog_stats` and `fog_occluders`). `BattleFrameOptions.fogGeometry` keeps only the shader text's `tile_px` and `radial_bins`. `setFog` and `FogInput` are gone. The tile lists are made at a size's first fogged frame.
- **Native** (`native/client/src/fog.rs`): `Fog::prepare` uploads the frame's `FogUpload`. The `FogSight` pass runs `fogTerrain`, `fogMerge`, `fogWhole` and `fogCull` after the depth prepass. Every fogged material binds the fragment group of what it is (`fog::Layer`, `modelFog.ts`'s rule):
  - units are never fogged;
  - a corpse is fogged as the ground under it;
  - the static world, scenery and the skirt are fogged face by face;
  - the terrain, grass and backdrop are the painted ground;
  - water is painted faces;
  - glass takes no fog.

  `Environment` no longer owns a fog group. A replay draws fog where the manifest lists the `fog` layer.
- **A fog reel:** `/graphics-test?fog=1` films with the side's fog and lists `fog` in its layers, and `PARITY_FOG=1` passes it from `native-parity-capture`. `bareTerrain`'s playable rectangle is now `[x, y, width, height]`, as the simulation's is.
- **Deleted TypeScript:** `fogInputs.ts`; `fogVisibility.ts`'s slots, rebuild picks, horizon, whole and affected-eye records, its occluder grid and its parameter packing; `fogVisibility.test.ts`; and the fog derivation in `useBattleSession`.

### Evidence

- **Equality, before deleting the TypeScript.** A scratch Vitest (`throwaway/fogsight/fogDump.scratch.test.ts`) ran the base's `createFogVisibility` on a recording device. On each publication it set the `FogInput` the page built, then encoded a frame, plus a second frame after every third publication. It recorded every buffer and texture write (FNV-1a), the full `FogParams`, every dispatch and every clear. It covered three runs:
  - paris-corner's whole capture: 181 publications, 241 frames;
  - market-town's capture window: 7 publications, 9 frames;
  - the fog labs' street (blue's encounter, 180 ticks): 240 frames, with garrisons and a knowledge change.

  `tests/fog_sight.rs` replays the same streams through `Presentation` (`set_ground`, `set_fog`, `push_publication`, `prepare_fog`), and every frame equals the browser's: params, eyes, rebuild records, occluders, foliage, the `wholes` words, the pass rows, the workgroups and the clears. It was falsified once by moving the horizon sectors' margin. The vectors are `fixtures/fog-sight-vectors.json` (126 KB) and the street's publications (`fog-street-publications.u32`, 204 KB).
- **The knowledge rules.** The vectors' streams have no fallen or destroyed known props; two falsifications passed the vectors. So the rules have their own tests:
  - Rust unit tests: a fallen map prop leaves and its remains take its place, and a destroyed or non-occluding known prop never occludes. Ported from `fog.test.ts` and `fogVisibility.test.ts`, as are the grid lookup, the candidates' order, the whole-fog pairing, the west wrap, the garrison's eyes and the immediate rebuild of touched maps.
  - `buildingKnowledge.test.ts` now asks a Rust presentation for each side's occluders over a shelled compound: blue sees the remains, red the standing building.
  - `fog.test.ts`'s metamorphic test (a wall the side has not seen never reaches its fog) runs through Rust.
- **Scenes:** `fog-look` (8 checks), `fog` (9) and `facade` (13, its fog check included) pass after merging main.
- **Native plates against a fresh fog control.** Neither reel shows fog, so the control is the reel filmed with fog on: `throwaway/native-parity/2026-10-12T19-26-40-508Z` (`PARITY_FOG=1`, both scenes), captured from this tree. Native replays it into `throwaway/fogsight/native/`; pairs and diffs are in `throwaway/fogsight/cmp/`.
  - Model, scenery and effect counts equal the browser's on every paris-corner plate. On market-town, only plates drawn on a tick boundary differ in effects, as in step 6.
  - Fog shows on paris-corner 1161 and 1509. Native's fog-on plates differ from its own fog-off replay there by distances of 0.062 and 0.091, and by less than 0.007 elsewhere.
  - Against the control, paris-corner 1509 has a distance of 0 and 1161 of 0.019. The unseen street, the fogged facades, the hatched balcony undersides, the far building fogged whole, and the unseen patch with its rim lie in the same places with the same look.
  - The unprimed critique (five pairs) agreed. Every difference it named is the props native does not draw yet (cars, lamps, bollards, hedges): `props20b` landed during this step, and native's model layer leaves their rows (`draws` 7 and 8) to a later step. On market-town 5008, rim patches show where the browser's hedges stand. It found no defect in the hatching.
  - **One difference that is not fog:** on paris-corner 152 and 1161, native casts a large building shadow across the road that the browser's plate lacks. Native's plates are the same with fog off (distance 0.002), and the browser's fog-off plate shows the same lit road. It is a native-against-browser shadow difference, recorded for step 8's gates.
- **Browser frame time** (`graphics-fps`, market-town, production, headless, under the GPU lock; base `c34a9f1df` against this branch, interleaved; the reel draws no fog):

  | Run | Load average | avg FPS | 1% low | p95 ms | GPU ms |
  | --- | --- | --- | --- | --- | --- |
  | base | 13.6 | 60.1 | 21.1 | 25.42 | 11.03 |
  | branch | 9.8 | 59.6 | 24.0 | 33.33 | 12.04 |
  | base | 6.0 | 66.4 | 30.3 | 25.02 | 10.25 |
  | branch | 6.7 | 67.0 | 36.9 | 25.00 | 10.45 |

  Level within the pairs' swing. The machine was busy with other sessions, so these are not quiet-machine numbers.
- **Gates:** `cargo test` for `presentation`, `pipelines` and `native-client`; clippy `-D warnings` on those and `presentation-wasm`; `cargo fmt --check`; the web typecheck; oxlint (no errors; the warnings are main's); nine web test files (51 tests).

### Code

Net lines against main, without comments and blank lines (`sight.rs`'s in-module tests counted as tests):

| | Production | Tests |
| --- | --- | --- |
| Rust | about +1,940: `fog/sight.rs` about 1,000 (rustfmt's layout), native `fog.rs` about 400, the WASM binding 130, the presentation, `Source`, records and native wiring | about +570 |
| TypeScript and TSX | −629 | −320 |
| JS (`native-parity-capture`) | +1 | |

Fixtures: 330 KB of vectors and publications. Generated: `records.gen.ts` (+133). The step adds Rust and removes TypeScript, so the browser's renderer only uploads what it is given.

### Decisions

In `choices.md`, "Slice 25, step 6b".

### Next pickup

- Native draws the props' batches (`draws` 7 and 8): the critique's differences are all props. Bind each through `Layer::Faces`, or `Layer::PaintedFaces` for one movers stand on.
- Step 8's gates. Look into paris-corner's building shadow on the road (152, 1161), which native casts and the browser does not.

## Step 5c: the props, and the static batches held between frames (2026-10-12, branch `nativeprops`)

Native draws the map's props and the known props from `Frame`'s static batches, as the browser does since 20b step 5. It now holds every static batch between frames rather than resending them.

### The contract

- **`PropInput` is read, not derived.** `Art.size` and `Art.radius` become `Art.bounds` (`{min, max}`: the appearance's first state's bounds), and `stand_in` is the stand-in kit's first state's bounds. Rust takes the drawn size from them (`world::Corners::bounds`, `bounds_size`, which moved from `presentation.rs` to `world.rs`). The page's `propArt` passes `bundle.states[0].bounds` and no longer calls `boundsSize`.
- **`files::ground(root, map)`** builds the map under the rules of the battle fought on it: the game's catalog plus the units of the map's category (a menu map takes the menu's, a test map the tests'). `MapGround` gains `layout` (the world layout its props are placed under) and `family`. Under the game's catalog alone, the layout named no menu unit, so a menu tank's wreck had no art and drew as the stand-in box.
- **`files::prop_art(root, family, wearing)`** is `PropInput` as a battle page wearing `wearing` installs it (`catalogLoadNames`, `familyLooks`, the stand-in kit). It reads the catalog's footprints, paints and wrecks, and each look's bounds from the head of its bundle (`library::state_corners`, which never inflates the meshes).
- **`Replay::open`** places the props after the scenery: `place_props(ground.props, layout, prop_art(.., the manifest's unit appearances))`.
- **Native's model layer** loads rows with `draws` 7 and 8 as props. They draw through the model pipelines under fog's `Faces` layer, or `PaintedFaces` for a surface movers stand on. Like a unit, a prop casts one tier coarser from its view batch; a building's module still casts only its own caster batches.
- **`statics.rs` holds the static world's batches** (the buildings' modules, the props, the scenery). Each `placed` row's batch has its own buffer, keyed by `Batch.key` and sized to a power of two of records. A batch is written only when the frame writes it, and dropped when the frame stops listing it, as the browser's layers hold theirs. Models and scenery draw from these buffers. The frame's instance buffer now carries only what the frame wrote. A batch named unchanged that native does not hold is refused by name.
- **The replay resends only when it jumps.** `Replay::frame(.., drawn)` asks for every static batch when a drawn frame follows an undrawn one, which is a plate replay skipping to its next plate. A drawn frame after a drawn one, which is every bench frame after the first, takes only what changed.
- **Counts:** a plate's `instanceCounts[].props` (by appearance and tier) is checked beside the models, scenery and effects. The bench's `meanDrawn` gains `props`.

### What draws, and what is left

- **Draws:** everything the browser's frame lists of the static world: buildings, scenery, the map's props (parked cars, lamps, bollards, bins, utility boxes, fences, walls, hedges, benches, resting wrecks) and the known props (rubble, wrecks, shoved bodies, the stand-in box for a kind with no art).
- **Left:** the painted ground marks, which a replay has no UI to paint. The step 8 gate itself, three quiet runs a side against the browser, is still open.

### Evidence

Controls were captured from this branch's tree: `throwaway/native-parity/2026-10-12T19-32-02-543Z`, before the merge of `389bf9b78` (fog's sight). After the merge, with fog off, the control is `20-07-17-354Z`; with fog on (`PARITY_FOG=1`), it is `20-11-12-980Z`. All three pass `native-parity-capture`. Plates, pairs and benches are in `throwaway/nativeprops/` (`cmp/`, `m/`, `bench/`).

- **Counts:** props per appearance and tier, scenery, and models are equal on all 24 plates of every control. Effects differ only on plates drawn exactly on a tick boundary (step 6's capture rule), with one exception: after the merge, three market-town plates late in the reel are off by one smoke puff. Main's own native binary shows the same miss on the same control, so it is main's, not this step's.
- **Plates** (`compare-screenshots`, grayscale MAE):

  | | before (step 5b) | now |
  | --- | --- | --- |
  | market-town | 0.05–0.56 | 0.02–0.10 (fog on 0.02–0.11) |
  | paris-corner | 0–0.74, and 4.54 on the parked-car street | 0–0.057 (fog on the same) |

  The parked-car street is plate 234 in this control. It is 0.057 now, and its only differing pixels are the tank's road wheels and distant soldiers.
- **Unprimed critique** (all 24 pairs, amplified differences, 3× crops of street level): no prop difference anywhere. Every car, fence, wall, hedge, lamp, bollard, wreck, box, piece of rubble and every shadow matches. The only differences it found are the tank's road wheels, soldiers' poses and sub-threshold noise. The defects it found are the same on both sides: plate 648 is inside a fireball, and the mortar lines on 1301 alias.
- **Paris-corner's "building shadow on the road"** (plates 152 and 1161 of another control) is the props. On main, native draws none of them. The road's real building shadow is visible where the browser draws parked cars and their shadows over it. Main's binary on this branch's controls gives MAE 0.25–0.74 on the same shots (583, 809, 1105). This branch gives at most 0.002 there, fog on and off. This branch with prop drawing switched off reproduces main's numbers exactly. No caster, cull or stale cast list is involved.
- **The bench** (release, under the GPU lock; `before` is the props with every static batch resent each frame, `after` holds them). The machine was busy with other lanes, so absolute numbers are higher than the quiet reading above. Two interleaved rounds were run:

  | scene | round (load, 1 min) | | frame ms p50 / p95 | CPU ms p50 / p95 | CPU prepare p50 | GPU ms mean | instance bytes a frame |
  | --- | --- | --- | --- | --- | --- | --- | --- |
  | paris-corner | 1 (7.5 → 6.2) | before | 9.64 / 15.57 | 3.55 / 4.97 | 0.67 | 10.10 | 122,518 |
  | | | after | 10.29 / 16.35 | **2.20** / 3.67 | **0.15** | 10.64 | **996** |
  | | 2 (11 → 22) | before | 14.10 / 47.02 | 4.27 / 44.04 | 0.83 | 14.89 | 122,518 |
  | | | after | 10.56 / 20.91 | 3.72 / 15.71 | 0.33 | 11.38 | 996 |
  | market-town | 1 (6.2 → 3.4) | before | 19.08 / 31.08 | 4.13 / 8.35 | 0.96 | 18.15 | 326,071 |
  | | | after | 14.78 / 20.33 | 3.29 / 11.48 | 1.11 | 13.61 | 194,095 |
  | | 2 (3.5 → 11) | before | 16.53 / 24.13 | 5.37 / 13.80 | 1.16 | 15.22 | 326,071 |
  | | | after | 14.31 / 20.31 | 4.94 / 11.87 | 1.56 | 14.13 | 194,095 |

  - On paris-corner, the upload falls from 120 KB to 1 KB a frame, and the CPU frame p50 falls by 0.6–1.4 ms, mostly in preparing.
  - Market-town's camera never stops, so its walk rewrites lists most frames: 190 KB a frame is still written. Its CPU frame is about 0.4–0.8 ms lower, within this machine's noise.
  - GPU time is load, not the change.
  - The gate's comparison with the browser still needs quiet runs.
- **Tests:**
  - `cargo test` passes for `presentation` (179), `pipelines` and `native-client` (all targets, with `assets/runtime/audio` pulled for `audio.rs`).
  - `native-client`'s new `tests/props.rs` holds three things: the paris-corner replay's prop rows include the menu tank's own wreck, only Paris's regional looks are installed, and the layout names the menu's units. That last test was falsified once, by building under the game's catalog alone.
  - Clippy `-D warnings` and `cargo fmt --check` are clean.
  - The web typecheck, oxlint (no errors) and oxfmt are clean. `kitsOnRequest.test.ts` passes.

### Code

Net lines against main, comments and blanks excluded:

| | Net |
| --- | --- |
| Rust, `presentation` | +122 (`prop_art` and the header reader; `Corners`; the props' input) |
| Rust, native | +167 (`statics.rs` 90; the models' props, counts and held runs; the scenery on held batches; the replay's jump rule) |
| Rust, tests | +53 |
| TypeScript | −1 |

### Decisions

- **The art a prop is fitted from is read in Rust from the files**, the way the scenery's kinds and the buildings' kits are. The one derivation `propArt` held, the drawn size of a bounds, moved into `PropArt::new` first, so both clients now hand over plain readings.
- **A map's battle is fought under its category's catalog**, the content owners' rule in the root README. Native has no session to ask.
- **Native's held statics live with the models**, which own the instance layout. The scenery reads them through `Models::statics`. That gives one holder for the static world, where the browser has one per layer.

### Next pickup

- Step 8's gates: three quiet runs a side, native against the browser, on both scenes, release build.
- Main's late smoke puff (market-town, between ticks, off by one) is an effects lead, not this step's.

## Effect counts equal on every plate (2026-10-12, branch `effectcounts`)

Step 6 left plates drawn on a tick boundary with different effect counts, and step 5c one extra smoke puff late in market-town. Both had one cause, and neither was native's.

### The cause

The page's presentation ran at a tick rate that was not the simulation's. The sim worker reported its rate as `1000 / tickMs`, which is 29.999999999999996, not 30. The page built its `Presentation` from that, while the capture recorded the rules' 30, and native and the WASM replay used 30. Every effect's start and end (`tick · dt`) and the interpolated clock were therefore a few ULPs apart. Between ticks that never mattered. On a tick boundary it decided whether the next tick's tracers and flashes had started and whether the last tick's impacts had ended. Late in market-town it decided whether a puff's lifetime had run out.

How it was found: the page logged every call into its Rust presentation, and every frame's publication, time and camera. All of them equalled the capture. Replaying the page's own frame sequence through the browser's WASM still differed on 1,050 of 6,947 frames, so the missing input was not in the per-frame calls. Shifting one boundary plate's time showed the page behaving as if its clock was a few ULPs early, which pointed at the rate.

### The contract

- **The worker reports the rules' rate as given** (`authority.ts`: `ready.tickHz` is `rules.tick_hz`; `tickMs` is derived from it, never the reverse). `authority.test.ts` holds it (red before the fix).
- **The capture records the rate the page's presentation ran at** (`reelRun.ts` takes `tickHz` from the worker's `ready`), so the replay's `Presentation::new` gets the page's input. `validatePresentationCapture` already refuses a rate that is not an integer.
- **Native checks building modules too:** `instanceCounts[].buildings` (`module · 4 + tier`, the view batches) against native's model layer (`Models::drawn_buildings`), beside the models, props, scenery and effects. Falsified once by miscounting.

The capture scene, the frame records and native's replay are unchanged: every other input they carried was already exact.

### Evidence

- **The browser's own WASM:** replaying the page's whole frame sequence (8,115 frames, 1,018 on a tick boundary) gives the page's instance count on every frame but the last, which framed a publication drawn after the reel ended.
- **Fresh controls, both scenes:** fog off `throwaway/native-parity/2026-10-12T21-04-36-341Z`, fog on `21-07-59-103Z`. All four pass `native-parity-capture`. Native's report says "model, building, prop, scenery and effect counts: equal to the browser's on every plate" for all 48 plates. Thirteen of them are on a tick boundary (paris-corner 3 and 1, market-town 5 and 4).
- **Plates** (grayscale MAE, control against native): market-town 0.016–0.096 (fog on 0.015–0.107) and paris-corner 0–0.049 (fog on 0–0.061). That is the same as step 5c. Native draws the same pictures, and the browser's change is a few ULPs of time.
- **Gates:** `cargo test` for `presentation` and `native-client`; clippy `-D warnings` on those and `presentation-wasm`; `cargo fmt --check`; the web typecheck; oxlint (no errors); `authority.test.ts` and `simSession.test.tsx`.

### Follow-up: native reads the page's numbers exactly (2026-10-13, branch `effectflake`)

After the rate fix, about one fresh capture in four still had a market-town plate whose effect counts differed (plate 4815, 38 running tracers against 58; in this lane's third capture, plate 6485, 108 streaks against 103). The next capture matched.

**The cause was native's JSON reader.** The page writes each frame's `simTime` as JavaScript does: the shortest text that reads back to its double. serde_json's default float reader is not correctly rounded and lands one ULP off on about 12% of frame times, and on about 7% of tick boundaries (`k / 30`). Plate 6485 was drawn at 3419/30, written `113.96666666666667`, which native read as `113.96666666666668`. One ULP past a boundary, that tick's effects end or start. Whether a plate fell on a misread boundary depended on the frame times, so it varied run to run. The page, its WASM and the capture were exact. The control's unfixed native differed on plate 6485; the fixed native, on the same capture, equals the browser on every plate.

**The contract:**
- **Native reads the numbers the page wrote with std's correctly rounded parse** (`capture.rs`'s `exact`): each frame's `simTime`, `reelMs`, `eye`, `target` and `up`, and the manifest's lens. The capture and manifest formats are unchanged.
- **Not serde_json's `float_roundtrip`.** Cargo enables a feature for every crate in a build, so it would also change how native's simulation reads the maps (market-town's has 9,164 numbers of 16 or more digits), while the page's WebAssembly simulation still reads them the default way. Both clients read the maps alike today.
- **Held by** `native/client/tests/counts.rs` (branch `replaytest`, which replaced the reader's own `109/30` test): native's replay of a committed count-only browser run (`PARITY_COUNTS=1`, the manifest's `frameCounts`: 1,387 frames of paris-corner's reel, 582 on a tick boundary) gives the browser's model, effect, building, prop and scenery counts on every counted frame. With serde_json's default reader in place of `exact` it fails on 86 counts, all on boundary frames (the first at 11.966666666666669 s). It runs in `cargo test`, with no GPU.

**Evidence:**
- Four fresh captures of both scenes before the fix: the third (`throwaway/native-parity/2026-10-13T20-22-01-640Z`) differed on market-town plate 6485 only. Fixed native equals the browser on it.
- Five consecutive fresh captures of both scenes after the fix (`20-29-10-910Z` to `20-41-59-679Z`): native's counts equal the browser's on all 120 plates. The fourth capture's plate 4693 also fell on a misread boundary (113.96666666666667); the unfixed native gives 108 streaks there against the browser's 103.
- **Gates:** `cargo test` for `presentation` and `native-client`, clippy `-D warnings` on both, `cargo fmt --check`, the web typecheck, oxlint (no errors).

### Step 6b follow-up: fog's CPU cost on the city map (2026-10-12, branch `fogperf`)

The slice 23 lane found `Presentation::prepare_fog` taking about 60 ms a frame on `city-contact`. There were three causes:
- the whole-fog pass searched every eye's structures again on every rebuilding frame (the TypeScript had kept the lists of eyes that had not moved);
- the broad phase deduplicated through a `HashSet` and sorted every result;
- every frame took the side's eyes and known props again, not only on a new publication.

**The fixes:**
- the lists are kept per eye position on the same grid;
- the broad phase uses bit sets read back in index order;
- a rebuilt eye's whole-fog search also gives its merge candidates, filtered by the same circle test;
- the sight is set only when a publication or a stand-in changes.

The uploads are unchanged: `fog_sight.rs` still matches word for word.

**Measured** (production build, `city-contact` short run):

| | Before | After |
| --- | --- | --- |
| `prepare_fog`, CDP profile, 10 s | 6,208 ms (about 68 ms a frame) | 2,017 ms (about 16 ms a frame) |
| Benchmark under the GPU lock (load average 8–10) | 9.1 FPS, CPU p95 164 ms | 12.6 FPS, CPU p95 102 ms |

What remains is inherent to the schedule the vectors pin: up to 48 eyes rebuilt each frame (their searches and the sectors' `atan2`/`asin`) and the whole-fog rows.

**Against the TypeScript it replaced** (the same 10 s CDP profile on `city-contact`, frames counted by `getCurrentTexture`):

| | Fog's main-thread cost in 10 s | Frames | Per frame |
| --- | --- | --- | --- |
| TypeScript (`b7096267f^`) | 3,814 ms | 164 | about 23 ms |
| Rust, after this fix | 2,321 ms | 258 | about 9 ms |

So fog now costs less than the TypeScript did. The rest of the main thread in both profiles is React and the battle overlays (`BattleView`, `battleOverlay`), not fog.

A rerun of the benchmark under the GPU lock (load average 5–8) passed every check, including "the simulation advanced in real time": 13.2 FPS, with a CPU p95 of 147 ms that swings run to run.
