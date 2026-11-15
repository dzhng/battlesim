# 00 Proof and probes

**Question:** what work does one admission do, and are the plan's unconfirmed
facts true?

**Depends on:** nothing.

## Contract

- Every admission reports `AdmissionWork`. The report is truthful: on a first
  admission it equals the known sizes of the art admitted.
- The swap's existing guarantees are pinned by tests before anything changes
  them: commit is atomic, and a stale, failed or dropped pending generation
  leaves the live one drawing the same picture.

## Seam

- [`Runtime::stage_assets`](../../../crates/renderer-core/src/runtime/assets.rs)
  returns its pending generation with the work done so far; `prepare` adds the
  card work. The report is counted where the work happens (decode, pack,
  upload, texture layer write, card bake, scenery placement). Counts and bytes
  only, never times.
- [`renderer-browser`](../../../crates/renderer-browser/src/appearance.rs)
  hands the report to the page, which adds its own counts (bundles encoded and
  sent, contents inflated, library installs) and publishes both with the
  `art committed` event
  ([viewportTrace.ts](../../../apps/battle-lab/src/viewportTrace.ts)).
- A scene reads the event's detail from the log call's second argument, not
  from its text (the text prints nested detail as `Object`).

## Verification

Write first, in a new `crates/renderer-core/tests/asset_generations.rs`, with
synthetic bundles (as `tests/bundle_admission.rs` builds them):

1. A first admission's report equals the fixture's known counts and bytes.
2. Stage and commit swap the picture; a pending generation that is dropped,
   stale or failed leaves the live picture and the live report unchanged.

Both are green today. They are the sacred contract every later slice keeps.
The first red test of the plan belongs to slice 03.

## Probes

Scratch work in `throwaway/`; record each answer in the
[ledger](../decisions.md) under "Confirmed during implementation".

- In the reference battle (link in the
  [freeze report](../assets/freeze-report-2026-11-14.json)): how long the card
  bake takes, how many rows it bakes, and what else fills the time from
  `battle staged` to `art committed`.
- Scenery and grass work at growth on the reference battle's map.
- The work report and long frames for one growth in `battle-growth` at the
  scene's own graphics profile and at high.

## What you can run or see

`battle-growth` prints one growth's work: today, the whole catalog.

## Delegated

Field names, how the report crosses the WASM seam, the probes' tooling.

**Fixed:** the report's meaning, where each count is taken, synthetic fixtures.

## Outcome

Done 2026-11-14. The probe's results are in the [ledger](../decisions.md): the
long wait is the card bake, and a superseded build is thrown away and redone.
Scenery's time at growth was not measured; slice 06 times it first.
