# One renderer

**Status:** implementing on [PR #16](https://github.com/dzhng/battlesim/pull/16).
**Updated:** 2026-10-28. Unified runtime and changing-frame parity pass; production player-feature
integration and final deletion/performance gates remain open.

## Architecture

One Rust `wgpu` runtime owns rendering for native and browser WASM. Both targets
consume the same presentation records, authored assets and pipeline metadata.
The runtime owns asset interpretation, GPU resources, pass order, materials,
models, shadows, terrain, scenery, grass, fog, effects, post and overlays.
Platform shells own only surface/device admission, asset transport, window/DOM,
audio and platform diagnostics. A browser-specific copy of native rendering
policy fails this architecture even when it looks similar.

`crates/renderer-core` is the runtime. Native filesystem loading and the desktop
surface live in `native/client`; fetched bytes and the WebGPU canvas live in
`crates/renderer-browser`. `presentation` remains the sole authority for poses,
visibility, tier/caster selection and static batches. Rendering must preserve
battle digests and publication layouts.

The browser exports presentation, rendering and the existing sound bindings from
one WASM module. Rendering synchronously borrows the actual Rust presentation;
it does not copy canonical frames or reconstruct knowledge through JavaScript.
The simulation worker remains separate. All browser classes use one generated
module/initializer, and JavaScript reacquires typed views after allocating calls
because rendering can grow that same memory. The core runtime owns resource
composition and preparation as well as pass encoding.

TypeGPU is a temporary control and shader migration input. After parity, delete
its browser runtime, duplicate pass/resource owners and dependency. Retaining a
build-only authoring dependency requires its own evidence that the tooling is
simpler. A permanent mixed runtime is not the target. Delete the standalone
production-shaped Rust graph as well.

The benefit being tested is **less total code and fewer owners across both
platforms**. Equal or slightly worse frame time is acceptable when the final
architecture removes substantial complexity. A smaller separately written Rust
renderer cannot establish that benefit.

## Next Agent Prompt

Continue with [slice 04](slices/04-player-features.md). Native and browser now call
one core Runtime for composition, preparation, rebuilding and encoding; browser
presentation/audio/rendering share one artifact and actual Rust records. Canonical
copy/upload/recovery protocols and the standalone presentation artifact are gone.

Priority order:

1. Finish the current player-feature checkpoint: paint/x-ray/high-air and actual
   lab poses/impostors now share core owners; all eight browser inspector views
   match. Query snapshot/concurrent-render and fresh visual checks pass. The tiny
   native delta is isolated to removal of the obsolete terrain depth workaround.
   Integrated review's fog recovery, lab card eligibility and Metal paint-clear
   findings are fixed with focused regressions. Checkpoint before widening consumers.
2. Finish proxy/workbench/sheet admission, quality and exact diagnostics, then
   replace the application's TypeGPU canvas interface. Read the scratch lab/model
   contract map rather than retaining a separate lab renderer. A nonempty browser
   high-air composition and appearance-generation races remain explicit gates.
3. Run the [migration/deletion verdict](slices/05-migration.md): final three-target
   workload/resolution measurements and source/owner accounting. Delete TypeGPU
   and the standalone Rust graph; run repository closeout once, review and archive.

The runtime checkpoint preserves the matched frozen street and all six native
plates byte for byte against their incumbents, including real device-loss pixels.
The dynamic browser pair consumes 120 captured frames / 60 publications and matches
final canonical state and source pixels essentially exactly. Detailed CPU/GPU,
artifact/memory limits and all failed trials live in research. Independent Codex
review found no actionable correctness issue; local cleanup refreshed views after
allocating stats calls and freed the temporary WorldView.

The default eleven-row scene remains green. Keep its controls while completing
parity. Capture final canvas pixels before destroying either backend's device:
a compositor screenshot after disposal may show the previous frame. Prefer the
retained native-resolution source pixels as the renderer comparison, with CSS
captures retained for composition evidence.

## Verification contract

The production acceptance workload is one frozen presentation publication,
camera, viewport, quality, simulation time and authored asset generation in the
TypeGPU browser control, shared Rust browser candidate and shared Rust native
control. Capture both the whole image and local material/shadow landmarks.
Inspect screenshots and get an unprimed critique; never bless missing content
because full-frame image distance improved.

Report startup and asset transport/decode separately from resize, surface
recovery and real device-loss recreation. CPU encoding and queue drain are
separate measurements; queue drain/compositor turns are not GPU timestamps.
Record submitted geometry/draws/passes, resident allocations, binary/download
size, native/browser shell LOC and shared core LOC. The final complexity result
must include deleted owners, not just added Rust files.

The normal `one-renderer` scene retains all ownership controls. Its capture mode
is a focused development experiment over the same Paris fixture in production
BattleFrame and the shared Rust browser core. It does not replace normal
regression coverage or prove visual parity automatically.

## Evidence and research

[Research](research.md) owns the hypothesis queue, attempts, measurements and
limitations. [Choices](choices.md) owns decisions made where the spec was silent.
[Slice 02](slices/02-verdict.md) records the shared-frame checkpoint; slices 03–05
own the remaining runtime, player-feature and migration gates. Scratch
screenshots, logs and raw measurements stay under ignored `throwaway/`.

Established before the current pass:

- Browser `wgpu`/WASM submission works; exported WGSL and bounded terrain/model
  transport controls match their browser counterparts.
- Shared production frame encoding, actual GPU owners, and resize/device-loss
  seams compile on native and WASM. Optional GPU timestamps and exact owned
  allocation snapshots support the matched measurements.
- A captured publication reaches the shared browser shell, including dynamic
  model, terrain, effect and caster records. Earlier dynamic-only admission and
  synthetic controls are not full production pixel or performance evidence.
- Native replay/count contracts pass; production feature coverage and the final
  migration gate remain open.

The shared-frame checkpoint matches the captured browser street essentially byte
for byte and preserves all six native replay plates byte for byte. Fresh critiques
found no candidate-specific regression. The default eleven-row scene passes.
Device recreation now preserves complete frame pixels without the harness
manually replaying scene/overlay uploads; its overlay-loss regression was proved
red then green. Detailed development timings and their limits live in research.

## Global TODO

- [x] Prove browser Rust WASM backend and four ownership variants — [slice 00](slices/00-wasm-backend.md).
- [x] Prove bounded real terrain/model transport and publication input — [slice 01](slices/01-real-frame.md).
- [x] Share actual production frame encoding and preserve the native/browser captured controls — [slice 02](slices/02-verdict.md).
- [x] Consolidate the runtime and use direct presentation borrowing in one browser module — [slice 03](slices/03-runtime.md).
- [ ] Complete browser player-feature parity — [slice 04](slices/04-player-features.md).
- [ ] Clear final lifecycle/performance gates and delete TypeGPU/duplicate graphs — [slice 05](slices/05-migration.md).
- [ ] Review the full implementation, run repository closeout, consolidate choices and archive.
