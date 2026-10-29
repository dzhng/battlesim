# One renderer

**Status:** implementing on [PR #16](https://github.com/dzhng/battlesim/pull/16).
**Updated:** 2026-10-28. Production integration and matched browser performance pass;
comparative startup is measured; browser regressions and final closeout remain open.

## Architecture

One Rust `wgpu` runtime owns rendering for native and browser WASM. Both targets
consume the same presentation records, authored assets and typed pipeline catalog.
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

TypeGPU's browser runtime, shader authoring dependency, expanded exports and
standalone production-shaped Rust spike are deleted. Authored WGSL has one shared
composition owner and typed Rust declarations own pipeline state. Historical
controls and raw measurements remain ignored evidence, rather than production
compatibility paths.

The benefit being tested is **less total code and fewer owners across both
platforms**. Equal or slightly worse frame time is acceptable when the final
architecture removes substantial complexity. A smaller separately written Rust
renderer cannot establish that benefit.

## Next Agent Prompt

Finish the [migration/deletion verdict](slices/05-migration.md) on PR #16.
The shared renderer, TypeGPU deletion, authored WGSL factoring, retained lifecycle
gates and whole-spec independent review are implemented. Renderer deletion and final source/interaction repairs are ready for checkpoint. The separate HUD coherence fix is commit `d9622d390`; last pushed
renderer checkpoint is `69a01c6e4`.

Current pickup:

1. Freeze a clean startup cohort for the reviewed candidate, then reconcile final
   accounting and claims. Bundle handles are shared through admission; US startup
   capacity is about 1.89 GB. A separate long-battle source transition peaks at
   3.59 GB after removing duplicate asset/world admission. Keep those quantities
   separate. Prior startup cohorts with compilation overlap remain development
   evidence. Frozen reel CPU/GPU results predate the final source repairs.
2. Finish the deletion checkpoint through hooks. Source retention, successful
   interaction snapshots and coherent frame-boundary source reconciliation are
   implemented and observed red/green. Independent review's pose-only framing
   finding is fixed; a second redraw-cleanup regression is fixed. TypeScript and
   scoped ESLint pass. Latest viewport suite: 27 pass; source/kit/regional and
   replay-refusal focused suites also pass.
3. Consolidate choices against shipped code, reconcile all gates, archive with
   fresh claim audits, push and immediately link PR #16. Do not merge. Full Rust
   workspace passed in 1881.29 s; the complete web run passed 1295/1296 tests,
   with its sole replay-setup import failure passing unchanged in focused and
   independent reproductions. Commit `1be68209e` adds network diagnostics and
   passes its focused replay-refusal test. Reuse valid expensive coverage.

Final browser coverage: street's vehicle tour, roster's four approved pictures,
frame's three approved pictures, picking, resize/device recovery, allies and
ally-help pass. The stale street proxy assertion now checks reserved proxy names
in the active canonical catalog. The earlier retained-generation attempt reached
WASM's ceiling because eager asset admission overlapped a full renderer rebuild;
the serial tracing and repaired run live in `ally-memory-{serial,traced,reconciled}/`.

Resolved closeout findings live in [research](research.md). The army digest pin,
readout layout barrier, brief allied-combat sampling and covered ruler journey
have separate committed repairs; the ruler's real contact right-click passes.
Air-hover's fresh visual concerns also occur in original TypeGPU. The roster's
extra dust was traced to the original browser's repeated world reloads: a
TypeGPU control fixing only its unstable faction dependency produces the shared
renderer’s exact effect records at the same observation/tick. Only the order
picture changes intentionally; capture time is now explicit, other pictures and
all thresholds stay intact. These are corrected presentation/test contracts,
not evidence of changed simulation outcomes.

The shared shader factoring preserves all64sources after whitespace normalization.
Maintained production-source accounting last measured +3599 noncomment lines (before final fixes),
including authored WGSL; refresh after final repairs. Generated exports are
separate. No maintained-source reduction has been established.

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

The comparison route and standalone ownership variants are retired after their
control build, source and raw results are frozen. Durable browser scenes and
native tests retain their relevant behavior and lifetime gates through the
production runtime.

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
- [x] Complete browser player-feature parity — [slice 04](slices/04-player-features.md).
- [ ] Clear final lifecycle/performance gates and delete TypeGPU/duplicate graphs — [slice 05](slices/05-migration.md).
- [ ] Review the full implementation, run repository closeout, consolidate choices and archive.
