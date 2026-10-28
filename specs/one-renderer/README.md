# One renderer

**Status:** implementing on [PR #16](https://github.com/dzhng/battlesim/pull/16).
**Updated:** 2026-10-28. The architecture is settled; production parity and the
final deletion/performance gates remain open.

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

Continue from the shared production frame. Native and browser now call the same
concrete Rust frame encoder over the actual sky, environment, ground, grass,
scars, fog, model, scenery, effects and post owners. Callback stage adapters and
the browser's parallel ground/environment/fog implementations are deleted.
Preserve native behavior while closing the matched captured Paris visual gate.

Priority order:

1. Inspect the new captured `plate-240` pair and rerun native plates against the
   saved pre-extraction native control. The preceding valid comparison failed
   visual parity; research preserves its evidence. Green transport/lifetime
   counters never override missing or different rendered content.
2. Complete the production browser canvas/transport shell and the remaining
   shared features listed in slice 02. Preserve paint, pointer marks, x-ray,
   high-air composition and the existing diagnostic contracts when replacing
   BattleFrame; the native replay alone does not cover these browser features.
3. Compare matched visual, CPU, GPU timestamp and owned memory results. Prepare
   fresh fog/scar/grass records for each drawn frame. After device recreation,
   resend current knowledge from presentation; an old delta cannot recover it.
4. Measure the three production targets with matched inputs, separate cold
   startup from recovery, record CPU/GPU/memory/download costs, and account for
   code/owners after deletion. Delete the replaced graphs, run repository
   closeout once, review the whole change, and archive the spec when shipped.

A browser environment failure occurred during this continuation: a minimal
Playwright data page timed out without game code; a bounded GPU-disabled probe
and a Metal probe loaded, and the default browser subsequently loaded again.
Those failed runs establish no renderer timings. Preserve their distinction from
application failures if it recurs.

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
[Slice 02](slices/02-verdict.md) owns the migration/deletion gates. Scratch
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
- Native replay/count contracts pass. Full visual parity remains unproven.

The current model audit invalidates the old browser asset adapter as a final
implementation: first-node articulated geometry, fallback static states and
partial texture-array uploads differ from native. Reusing native admission is
both the parity fix and the complexity reduction.

## Global TODO

- [x] Prove browser Rust WASM backend and four ownership variants — [slice 00](slices/00-wasm-backend.md).
- [x] Prove bounded real terrain/model transport and publication input — [slice 01](slices/01-real-frame.md).
- [ ] Move every production GPU owner into the shared runtime — [slice 02](slices/02-verdict.md).
- [ ] Clear matched production-frame, performance and lifecycle gates on both targets — [slice 02](slices/02-verdict.md).
- [ ] Delete TypeGPU runtime/dependency and duplicate production-shaped graph — [slice 02](slices/02-verdict.md).
- [ ] Review the full implementation, run repository closeout, consolidate choices and archive.
