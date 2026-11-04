# Performance across gameplay views

**Status:** shipped; accepted scope and measurement limits recorded below.
**Closed:** 2026-10-21.

## Purpose

The performance work makes a local gameplay view pay for the geometry and shadow
work that can affect that view, while preserving side knowledge and authored
geometry. Coarse chunks and forest motion envelopes can over-admit geometry;
this is bounded spatial selection, not a guarantee of constant whole-frame cost. It covers Metro XL seed 4 at the production close,
tactical and wide camera scales (65, 260 and 2,400 m), with stationary, moving,
live and late-state evidence owned by the measurement harness.

The durable contract is bounded local work, not a universal FPS promise. A fixed
camera may still see a large horizon or many legitimate casters. Counters describe
actual submissions and executed selection work; installed capacity, cached answers
and partial pass counters are not interchangeable with frame time.

## Measured outcome

There is no net fixed-view GPU speedup in the retained three-view comparison.
Repeated paused old-control/candidate medians were 5.03/5.63 ms close,
5.50/5.74 ms tactical and 32.64/34.41 ms wide. Correct caster coverage adds
net 3.07 million shadow triangles and 53.47 MiB of buffers at the wide view.
These are costs, not improvements to average away.

The demonstrated benefit is bounded selection and receiver work. In the
terrain-only matched local-view growth fixture, main-thread windows at the largest
remote extension fell from 4.76–4.90 ms to 0.34–0.64 ms. Static-population tests
prove structural selection contracts separately. Neither establishes constant
whole-frame cost for arbitrary battles. The memory and conservative over-selection
tradeoffs are explicit in [choices](choices.md).

## Why this shape

Terrain, water, boundaries and static populations use shared spatial bounds and
range selection. Whole triangles retain their authored vertices and attributes;
selection skips rejected hierarchy branches and merges adjacent ranges. The Rust
presentation owner supplies the same records and visibility decisions to browser
and native clients. See the [shared spatial owner](../../../crates/presentation/src/static_world/index.rs),
[terrain partition](../../../crates/presentation/src/static_world/ground/partition.rs),
and [shared range draws](../../../crates/renderer-core/src/ground.rs).

Camera visibility and shadow reach are separate contracts. Shadows fit local
terrain receivers, retain standing headroom and configured reach, and select
casters against the fitted cascade cameras. Four cascade lists route only to the
shadow images they can affect; offscreen casters remain eligible. Receiver fits
are reused while camera, terrain and light identities hold and invalidated when
those inputs change. See [receiver fitting](../../../crates/presentation/src/shadow.rs),
[shared cascade routing](../../../crates/renderer-core/src/frame.rs).

Forest knowledge changes filter one immutable hierarchy whose envelopes include
falling motion. Standing bounds still control standing detail; known falls select
fallen records and stumps. This avoids rebuilding all standing geometry when a
new fall is learned. Full known-fall snapshot reconciliation remains linear in
known falls and is deliberately reported separately from local selection.

The browser keeps existing publication and GPU lifetime contracts. Building
records are republished as one coherent packed set when their generation changes;
temporary storage is reused where its owner permits. Browser terrain draw and
receiver partitions remain separate allocations because no measured benefit has
justified a new cross-boundary lifetime owner. These are explicit memory/CPU
limits, not hidden promises of constant cost.

## Representation decisions

Low quality renders the 3D world at 0.75 scale while HTML HUD controls retain
normal resolution. Minimum world-stroke coverage follows render pixels while
authored widths and contact body radius follow CSS pixels. Existing Rust reuse and the shipped simplified 172-triangle
far car mesh remain in production. Walls, vegetation, planters and other classes
retain authored meshes where no simple, visually safe replacement earned its
maintenance cost.

The far-car card pilot is not part of the product. Surface-depth reconstruction,
coverage and native/browser asset plumbing were removed after the final candidate
retained pixels in the expanded camera states, but softness and temporal
appearance remained uncertain; the candidate was not qualified. Earlier filtered
failures caused by invalid buffer decoding are historical evidence, not the
reason to claim that all cards fail. Existing unit/corpse cards retain their
independent affine, lifetime and asset-identity corrections.

## Invariants

- Renderer changes preserve simulation outcomes, side knowledge and replay
  behavior. The separate native/WASM AI portability and static-obstacle rolling-turn
  recovery corrections are outside that rendering claim.
- Browser and native consume the same Rust presentation and shadow contracts.
- Camera lists never substitute for caster lists; visible ground may retain an
  offscreen caster.
- A rejected representation cannot make a gameplay object or its shadow vanish.
- Local selection may be conservative; it must preserve authored geometry and
  stable installed-buffer ordering. Terrain preserves authored triangle contents
  but groups triangles into spatial tiles, changing cross-tile source order.
- Receiver cache reuse is valid only for unchanged camera and installed inputs.
- GPU/resource measurements identify their scope and do not turn counters into
  causal frame-time claims.

## Dead ends

The per-sample card shader was rejected after fresh GPU comparisons; centroid and
filtered variants did not provide a qualified visual/performance result. A small
building-member caster refinement was also removed because the trials did not
establish a timing benefit. The
old camera-only shadow approximation is not a valid optimization because it can
omit an offscreen shadow contributor. Optional terrain-copy sharing and selected
list recycling remain deferred until measurements show their cost matters.

## Pointers and provenance

The code owners are the presentation static-world and shadow modules and the
shared renderer's frame/model layers. Focused
contracts live beside those owners; the three-view harness owns camera, phase,
submission and GPU-window measurement. Prior shipped context is in
[prior results](prior-results.md), [research](research.md), and [choices](choices.md).

The visual standard was the existing game at matched camera and battle state,
with authored geometry and legitimate offscreen shadows retained. Recorded game captures in
`throwaway/three-view/` are the baseline and comparison evidence for the three
production views; they are ignored scratch artifacts and remain outside the
closed spec. Approved resolution-regression references remain in
[the frame scene baselines](../../../web/scenes/baselines/frame/).

## Verification and limits

The complete Rust and browser unit suites ran once; subsequent fixture and
render-scale corrections passed affected checks. Full browser verification exposed
a static-obstacle vehicle recovery failure. Its focused regression, neighboring
movement/drive checks, and original browser helper-response scene now pass; the
last completed in 514 seconds. The post-correction simulation suite passed all
165 unit and 1,122 integration tests, plus its example tests.
Final mesh resource-reset checks passed; residency is high but stable across
resets. Native matched all submitted categories across 18 market-town plates;
visual review found no broad geometry or shadow defect, with tiny infantry
representation differences remaining between native meshes and browser cards.

All moving paths were collected on control and candidate. Review of actual tours,
excluding deployment setup, found no candidate-only visible geometry, shadow or
framing defect. Shared fog hatching reduces wide-view readability. Active phases
advance different tick counts in equal wall time, and diagnostic CPU work was not
consistently excluded from the candidate timing run, so those comparisons cannot
support an isolated moving speedup or regression claim. Preserve that limitation
unless a quiet matched repeat replaces them.
