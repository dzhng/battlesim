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
- **Code** (this pass): production −115 net. That is TypeScript −432 +154 and Rust +189 (the light's derivations, the two records and the binding). Tests +187.

### Decisions

- **A free `light_records`, not `load_world`.** See the contract above. 18b is amended.
- **Native Godot's own ports of these formulas** (`environment/light.gd`, `sky_model.gd`, `aerial.gd`) are frozen and leave with Godot at step 8. Their comments name the deleted TS.

