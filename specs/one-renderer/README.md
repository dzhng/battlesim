# One renderer

**Status:** implementation in progress on PR #16. Shared targets, pipelines,
depth, world, indexed model upload, and fog-mask resources now have one Rust
owner; production effects, post, fog compute, lifecycle, and TypeGPU deletion
remain.
**Updated:** 2026-10-27.

The first question is answered: Rust `wgpu` can run in browser WASM through
its `BrowserWebGpu` backend, attach to the same HTML canvas contract, consume
WGSL, and submit frames. The important question is different: can the existing
native `wgpu` renderer become one shared renderer core compiled for both native
and browser targets? The native client already owns the complete pass graph in
`native/client/src/`—frame orchestration, targets, shadows, world/models,
effects, fog, post, scenery and resource lifetimes—and already consumes the
browser's exported WGSL and presentation records. A second bounded Rust graph
is not the target architecture and its smaller LOC is not evidence of a win.

## Next Agent Prompt

Treat the current `renderer-wasm-spike` as a transport and harness probe only.
The first extraction checkpoint is now complete: `crates/renderer-core` owns
the shared target formats, target allocation and GPU buffer/bind helpers, and
 the native client consumes those owners. The next pass must move the actual
 pass modules into that crate, then add native and WASM platform shells for
surface/device/assets/audio concerns. Do not add another Rust pass graph.
Keep TypeGPU as the browser control until the shared native/browser core clears
the matched-image, startup, frame-time, resize, rebuild and device-loss gates.

## Decision map

The user delegated the spike choices: test all four useful ownership shapes and
let evidence decide. The spike therefore covers:

1. TypeGPU authoring and browser WebGPU submission (the current control).
2. Exported WGSL and direct browser WebGPU submission (no TypeGPU runtime).
3. The existing native Rust `wgpu` renderer compiled for browser WASM using the
   same exported WGSL and presentation records.
4. Rust base plus a TypeGPU overlay plane.

TypeGPU remains a temporary shader authoring source during this spike.
Rewriting the shader language is a separate build-tool decision, but TypeGPU has
no permanent renderer role. The final shipped architecture deletes the TypeGPU
runtime and dependency unless a dedicated build-only authoring spike proves
that retaining it pays for its complexity. The mixed result currently uses a
second canvas plane; same-device, same-texture interop is unproven and must not
be inferred from that result.

## Settled contracts and constraints

- Rust `presentation` remains the only owner of battle observations, frame
  records, culling, tiers, shadows and side knowledge. The spike renderer only
  consumes a canvas and draw inputs.
- Native already consumes the browser's exported WGSL and pipeline metadata;
  the browser target must reuse that native renderer core, not implement a
  second pass graph in `renderer-wasm-spike`.
- Platform-specific code may differ only at the shell: surface creation,
  adapter/device setup, runtime asset loading, file access and audio. Pass
  ordering, targets, bindings, resource lifetimes and presentation-record
  handling have one shared owner.
- The browser's existing TypeGPU frame remains a temporary control. Its real contract
  includes pass order, reverse-Z depth, targets, fog, shadows, effects, post,
  overlays, resource lifetimes and the `presentation-wasm` feed.
- The browser must remain shippable during any future migration. A measured
  backend experiment is reversible and does not change battle digests or
  publication layouts.
- The acceptance workload for the next slice is the same real frozen
  presentation frame rendered by the native renderer core in both targets, not
  an empty or synthetic camera. It must report startup, bundle bytes,
  CPU/frame, GPU duration, submitted work, memory and a matched image for both
  targets, plus the shared-core versus platform-shell LOC and complexity.

## The decision this spec must answer

The candidate architecture is **one Rust `wgpu` renderer core, two thin
platform shells**:

```text
presentation records + exported WGSL
              │
              ▼
     shared native/browser wgpu core
          ┌──────────────┐
          ▼              ▼
   native surface     browser WASM surface
```

The comparison is not “TypeGPU versus a separately written Rust renderer.” It
is:

| Question | Browser TypeGPU | Shared Rust `wgpu` core |
| --- | --- | --- |
| Production pass implementation | Existing `packages/battle-renderer` | Existing `native/client` core, extracted once |
| Browser target | Direct WebGPU/TypeGPU control | `wgpu` `BrowserWebGpu` WASM shell |
| Native target | N/A for the browser control | Existing native `wgpu` shell |
| Shader source | TypeGPU exports WGSL | Same exported WGSL/pipeline metadata |
| Main decision | Current baseline | Whether one core removes more code/drift than it adds |

The spike is successful only if the shared Rust core renders the same production
frame in native and browser builds without a second pass implementation. A
standalone `renderer-wasm-spike` graph may remain as a tiny transport smoke test,
but it must not be used for the complexity or migration verdict.

## Remaining implementation passes

There are four focused unknowns left. They are ordered so a failed assumption
stops the work before a full port is attempted:

1. **WASM compile boundary — checkpoint complete.** `renderer-core` now
   compiles for `wasm32-unknown-unknown`, while the native client still owns
   the desktop-only shell. The remaining blockers are listed below.
2. **Pass orchestration — checkpoint complete.** `renderer-core::encode_passes`
   owns ordered pass-name iteration and debug-group lifecycle used by every
   native pass. This proves the frame encoding path crosses the shared boundary
   without a duplicate graph.
3. **Pipeline/provider seam — checkpoint complete.** `renderer-core::pipelines`
   owns export parsing, layout/pipeline construction and validation from a
   platform-provided shader source callback. The native shell only reads files
   and retains its Windows cutout adaptation; a browser shell can fetch the same
   export and shader bytes.
4. **Depth-pass extraction — checkpoint complete.** The reverse-Z depth prepass
   now lives in `renderer-core::depth`; the native shell adapts its ground,
   scenery and model resources through `DepthInputs`. This is the first actual
   pass implementation shared by both targets.
5. **World/material extraction — checkpoint complete.** `renderer-core::world`
   now owns the HDR/fog-mask color attachments, sky/world ordering, reverse-Z
   depth load/store, layer/surface iteration and clear/load policy. The native
   shell adapts concrete ground, scenery, fog, models and sky resources through
   `WorldInputs`; normal and regional scar passes preserve their submitted work.
   The shared addition is 148 Rust lines; the native world module is 17 lines
   smaller net (122 added, 105 removed). Native compile and presentation tests
   remain green. This extraction adds no GPU work and no extra render pass.
6. **WASM renderer-core boundary — checkpoint complete.** The presentation
   filesystem modules are desktop-only under `wasm32`, and
   `cargo check -p native-client --target wasm32-unknown-unknown` now passes for
   the full package. The desktop binary shell is cfg-isolated (`main.rs`
   imports audio, replay, counts, `RUNTIME` and filesystem `Library::open` only
   on native), while `bundle::Library` has an explicit browser stub so the
   shared model/grass/scenery code compiles. The browser shell now also fetches
   the authored test-tank bundle, parses it through `Library::from_bundle`, and
   uploads its first texture. The shared model owner then draws an indexed tier
   from that parsed asset; production materials and skinning remain open.
7. **Shared model/lifecycle shell — model checkpoint complete.** The shared
   `renderer-core::model::Model` owner now expands indexed geometry, uploads it,
   and draws it from the browser shell. The shared row exercises a 37,002-vertex
   fetched tank and resizes 640×360 → 320×180 → 640×360 before measurement.
   Texture/effect assets, rebuild, surface loss and device loss remain open.
8. **Fog-mask extraction — checkpoint complete.** `renderer-core::fog_mask`
   owns the fog look/view uniforms, optional edge-distance passes, compose pass,
   rim pass, and attachment policy. Native wraps the shared owner; GPU work and
   pass order are unchanged. Fog sight/compute/material classification remain
   native-only and must move next.
9. **Effects and post extraction — checkpoint complete.** Flipbook/effect
   resource ownership remains shell-provided, while browser-safe effects and
   bloom/grade target/pass ownership now live in `renderer-core` and are
   encoded by the shared browser frame.
10. **Lifecycle extraction — checkpoint complete.** `renderer_core::TargetOwner`
   atomically swaps size-dependent targets and increments one shared generation;
   native `Passes` consumes it without changing GPU work. Fog sight/compute and
   device-loss recovery remain open.
11. **Fog compute and browser production frame — next.** The four fog compute
   pipeline bindings and dispatch labels now have a shared owner; move the fog
   sight buffers/bind groups into that runtime, then replace the browser
   fixture frame with the production shared frame.
   The browser shell now embeds the exact 63-pipeline export and every WGSL
   source through a generated build-time map; no filesystem access is added,
   and exposes an explicit validation method for building that export on the
   browser device. The one-renderer scene calls that validation before the
   shared row and records all 63 pipelines in its workload.
   Model material bind-group ownership is also shared now; native palette
   growth rebinds through `renderer_core::model_material`.
   The browser shell now also allocates the shared four-cascade shadow owner;
   caster routing and shadow sampling remain part of the production parity
   gap.
   The shared browser frame now allocates the real exported pipeline set and
   encodes shared fog-mask, post, and rim passes on every fixture frame. The
   fixture model/world draw remains the bounded input until production
   presentation records and materials are connected.
   Browser resize now uses the shared `TargetOwner`, rebuilds the shared fog
   and post resources with the new target set, and reports generation 2 after
   the scene's down/up resize round trip.
   The browser shell also binds the exported model layout through the shared
   material owner and reconnects the fetched albedo texture as a 2D array,
   reporting `sharedMaterials: true` in the run.
   The browser shell also reconfigures a lost/outdated surface through one
   recovery method; the scene now requires and records one recovery. Device
   loss is instrumented (`deviceLost: 0` in the run), but recreating the device
   and all resources is still an open gate.
12. **Three-way decision run.** Compare TypeGPU browser, shared Rust browser and
   shared Rust native on the same frozen presentation frame. Measure startup,
   CPU encode, GPU timestamps, memory, binary/download size, LOC, GPU object
   counts, screenshot distance and recovery behaviour. Only this run can decide
   whether the shared core is simpler overall.

## Evidence from the completed spike

The runnable surface is the `one-renderer` developer tool and scene:

- `crates/renderer-wasm-spike` compiles for `wasm32-unknown-unknown` with
  `wgpu` features `webgpu` and `wgsl`.
- `apps/battle-lab/src/routes/oneRenderer.tsx` runs eleven paths for 120 frames
  on one adapter.
- `web/scenes/one-renderer.mjs` checks completion, captures the eleven canvases,
  compares the control pixels, and writes evidence under `throwaway/`.
- The post-fix scene runs passed. The initial four micro-workload readings were:

  | Path | Setup | Encode | GPU drain |
  | --- | ---: | ---: | ---: |
  | TypeGPU authoring + browser WebGPU | 17.0 ms | 1.0 ms | 7.2 ms |
  | Exported WGSL + browser WebGPU | 1.2 ms | 0.4 ms | 4.9 ms |
  | Rust `wgpu` + BrowserWebGpu WASM | 2.1 ms | 1.0 ms | 8.1 ms |
  | Rust base + TypeGPU overlay | 3.4 ms | 1.3 ms | 6.7 ms |

  These are micro-workload diagnostics, not battle-frame performance claims.
  GPU drain is affected by browser scheduling and each row creates its own
  device; use the next slice's interleaved frozen battle run for a decision.

- Rust versus exported WGSL and TypeGPU versus exported WGSL both had mean
  RGB difference `0` in the captured canvas pixels.
- The shared browser shell currently emits 508,439 bytes of WASM plus 69,184
  bytes of generated JS. Its asset proof fetches 3,960,005 compressed bytes
  before decoding the authored test tank.
- `cargo check -p native-client --target wasm32-unknown-unknown` does not
  compile the existing native client: the `presentation` files feature reaches
  `sim::maps` and `sim::fixtures`, which are gated out on WASM. The native
  client also owns desktop window/audio dependencies. The spike avoids those
  boundaries rather than hiding them.
- The first screenshot review found a clipped layout and an indistinct mixed
  overlay. The harness now uses a taller viewport, eight result rows, and a
  visible premultiplied overlay marker; the final screenshot captures the full
  comparison and the overlay registration.
- The real terrain pair now uses the same `ScenePresentation` camera record and
  authored 10-float terrain mesh in both backends. Its captured mean RGB
  difference is `0`; this is a geometry/camera handoff proof, not full material
  parity.
- The authored model pair loads the existing `test_tank` runtime bundle through
  the catalog and gzip/texture codec, selects its authored LOD1 meshes and
  vertex colours, applies the bundle's resting articulated palette, and uploads
  the bundle's first authored albedo texture and UVs. Browser and Rust receive
  the same 9-float vertex stream, u32 index stream, 8-float placement/tint
  records, texture bytes and `Frame.camera` record. Its captured mean RGB
  difference is `0.049`, below the `0.1` scene gate.
- A representative 120-frame model run measured browser setup/encode/GPU drain
  at `76.6 / 0.9 / 14.0 ms` and Rust setup/encode/present-boundary at
  `7.2 / 23.3 / 0.2 ms`. Browser setup includes runtime bundle, texture decode,
  and persistent GPU buffer allocation; the Rust row reuses those browser
  arrays. These numbers prove the seam and upload shape, not whole-game startup
  parity.
- The model pair now uses a shared reverse-Z `depth32float` target with
  `greater` comparison for both terrain and model passes. This proves the first
  depth/resource-lifetime seam; production shadow cascades and fog depth remain
  open.
- Each model row now reconfigures its canvas down to 320×180 and back to
  640×360 before the measured frames. Both browser WebGPU and Rust `wgpu`
  complete the resize gate before drawing the captured workload.
- The scene now runs three 120-frame repeats with the row order rotated on each
  repeat, then reports the per-path average. The latest averages were:

  | Path | Setup | Encode | GPU/present boundary |
  | --- | ---: | ---: | ---: |
  | TypeGPU authoring + browser WebGPU | 6.0 ms | 0.6 ms | 5.8 ms |
  | Exported WGSL + browser WebGPU | 0.3 ms | 0.3 ms | 4.6 ms |
  | Rust `wgpu` + BrowserWebGpu WASM | 0.9 ms | 0.9 ms | 10.2 ms |
  | Rust base + TypeGPU overlay | 2.8 ms | 1.1 ms | 6.1 ms |
  | Real terrain + browser WebGPU | 6.4 ms | 5.9 ms | 4.0 ms |
  | Real terrain + Rust `wgpu` WASM | 2.3 ms | 4.7 ms | 4.9 ms |
  | Authored tank mesh + browser WebGPU | 17.7 ms | 0.4 ms | 12.6 ms |
  | Authored tank mesh + Rust `wgpu` WASM | 4.0 ms | 15.5 ms | 39.6 ms |

  The Rust row's present boundary includes compositor turns needed to make
  repeated WASM captures stable; neither boundary is a GPU-time claim.
- Fresh visual review confirms the real pair is byte-identical, while also
  calling out the limit of this pass: the fixture is a low-detail terrain patch
  with no units, shadows or material landmarks. That is intentional evidence
  for the handoff seam, not a claim of full battle visual parity.
- The latest unprimed review confirms the 1280×2300 capture contains all eight
  panels and table rows. The authored tank pair is readable and contained,
  though its dark navy shading is low-contrast against the muted terrain at
  full-page scale; the first three gradient controls remain intentionally
  featureless micro-workloads. No clipping or broken model edges were found.
- The fresh interleaved review found no tank clipping or depth-order defects,
  while noting that the near-black palette and hard diagonal terrain boundary
  make this fixture weak for material and scene-fidelity judgments.
- The production BattleFrame control runs the real light, fog, shadow,
  effect-atlas, target, post, overlay and static-prop workload on the frozen
  patch. Rust consumes the authored tanks, static prop mesh, fitted cascade
  matrices, resolved sun/fog records and a synthetic effect record through the
  WASM seam. The browser fixture's live effect count remains zero, so effect
  content is still an explicit input mismatch. The latest run is
  `154.0 / 32.8 / 140.0 ms` for browser setup, CPU encoding
  and queue drain versus `8.9 / 21.0 / 30.4 ms` for Rust. Rust executes 480
  shadow passes, 120 world/effect/fog/post passes and 1,560 draw calls, but
  reports three unsupported material channels and unsupported shadow sampling.
  The full-frame mean RGB distance is `47.03`. This is pass-graph and cost
  evidence, not a parity gate: the Rust image is visibly underexposed and
  omits BattleFrame material, fog, overlay and post behaviour.
- The shared native/browser row now fetches the real `test_tank` runtime asset
  through the browser shell. Its latest setup/encode/queue-boundary reading is
  `41.1 / 1.6 / 0.1 ms` for 120 frames. The parser sees 27 textures, 12
  materials, 98,021 vertices and 236,706 indices from 3,960,005 compressed
  wire bytes; it uploads the first authored texture and the shared model owner
  draws 37,002 indexed vertices. The row also completes a 640×360 → 320×180 →
  640×360 resize round trip. The screenshot is structurally non-empty but its
  top-down, low-contrast fixture does not claim production material, fog,
  shadow, post or overlay parity.

## Current verdict

The browser backend is feasible, and the exported WGSL and presentation
contracts are already shared with native. The bounded terrain/model checks pass,
but the standalone Rust production-shaped graph is not the architecture this
spec is meant to decide. It duplicates renderer ownership beside the existing
native `wgpu` client, so its smaller LOC and faster setup cannot answer whether
one Rust core is simpler overall.

The target is the shared Rust runtime, not a permanent hybrid. Keep TypeGPU only
as a temporary control and shader migration input while the Rust runtime clears
the gates. Then delete the browser TypeGPU renderer and dependency, unless a
separate build-only authoring experiment proves that retaining TypeGPU produces
enough value to justify its complexity. The current screenshot review remains
useful as a transport diagnostic: the standalone Rust image is visibly
underexposed and the fixture control has placeholder markers, clipped edges and
weak shadows.

## Global TODO

- [x] Add a Rust `wgpu` WebAssembly canvas backend — [slice 00](slices/00-wasm-backend.md).
- [x] Compare TypeGPU, exported WGSL, Rust `wgpu`, and mixed ownership — [slice 00](slices/00-wasm-backend.md).
- [x] Verify pixel equality on the spike workload — [slice 00](slices/00-wasm-backend.md).
- [x] Port one real presentation-fed terrain pass and its camera/mesh handoff — [slice 01](slices/01-real-frame.md).
- [x] Port one real authored model pass — [slice 01](slices/01-real-frame.md).
- [x] Run interleaved reel measurements with startup, bundle, GPU, CPU and reset gates — [slice 01](slices/01-real-frame.md).
- [ ] Move every production GPU owner into the shared Rust runtime — [slice 02](slices/02-verdict.md).
- [ ] Clear matched production-frame and lifecycle gates in native and browser — [slice 02](slices/02-verdict.md).
- [ ] Delete the browser TypeGPU renderer and standalone production-shaped graph — [slice 02](slices/02-verdict.md).

No compatibility layer or migration scaffold is approved. A future production
slice must either delete the replaced TypeGPU path or explicitly record why a
small mixed plane remains an owner with a removal condition.
