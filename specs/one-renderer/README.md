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

Continue the [migration/deletion verdict](slices/05-migration.md) on the existing
PR. Production viewport/workbench integration is implemented and reviewed: one
page GPU, canonical BrowserScene, atomic appearance admission and shared stats.
The newest checkpoint contains the 19-test admission lifetime corrections and
source-sharing memory reduction. The spec remains incomplete.

Current pickup and priority order:

1. Build the final matched TypeGPU-browser / shared-Rust-browser / shared-Rust-
   native cohort through actual production admission. Match 1280×720 and exact
   replay frame indices; measure frozen and changing input separately. Existing
   timing controls use 640×360 and direct Renderer installation, so they are
   historical evidence. The 43.2% CPU saving already used unified WASM and does
   not isolate an individual cause.
2. Preserve that control, then delete TypeGPU, its duplicate resource/pass owners,
   shader authoring dependency and standalone Rust graph. Extract CPU contracts
   and give WGSL/pipeline authoring one lasting owner before deleting exports.
3. Finish whole-spec review, full repository closeout, choice consolidation and
   archival. The general browser runner's oversized React measure diagnostic
   during the fog journey remains a closeout concern; the focused fog behavior
   probes pass after waiting for the actual presented tick.

Current raw protocol and deletion inventory are in ignored
`throwaway/final-perf/protocol.md` and `throwaway/deletion-inventory.md`. The
native lane is adding held-frame benchmark options; do not overwrite its work.

Production sheets and the workbench's latest-source races pass. Correcting the
old controls' missing canonical ground and lost startup world update closes both
visible differences without changing picture tolerances. Shared immutable model
geometry reduces the same generated skirmish's owned GPU payload from 2.670 GB to
1.224 GB with pixel-identical output. These are feature and memory results; the
final performance and complexity verdict remains open.

The code still contains TypeGPU controls and exported shaders. Deletion must
extract live CPU contracts first, give shared WGSL and pipeline metadata a lasting
owner, then remove the old graph and dependencies. Do not turn 64 duplicated
shader exports into manually maintained copies. The application no longer imports
the old BattleFrame owner outside the retained comparison route.

Detailed measurements, current failure and raw evidence live in
[research](research.md); scratch implementation protocols stay in `throwaway/`.

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
