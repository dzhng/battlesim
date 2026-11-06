# 16 — Map manual operation lifetime

**Status:** complete. **Audit families:** 76. **Dependencies:** 15.

## Contract and question

Can manual inspect/compare/sample/review/save work own cancellation and background resumption in one place?

Current owners/consumers: [MapWorkbench.tsx](../../../apps/map-workbench/src/MapWorkbench.tsx), [runner.ts](../../../apps/map-workbench/src/runner.ts), [sample.ts](../../../apps/map-workbench/src/sample.ts).

## API seam and target shape

Keep a map-local operation controller with one active manual record {kind, controller, captured input/artifact, cancellation generation}. The reducer from slice 15 projects progress and result. Existing LatestRunner remains background owner; manual work awaits its cancel completion rather than starting competing work. No shared job framework.

## Transitions and invariants

Start cancels the prior manual operation and background runner, captures current inputs, executes existing API, and publishes only while owned/alive. Finish may resume valid latest background draft only under the existing cancellation-generation rule. Explicit Cancel work retains sample outcomes and must not restart generation. Departure cancels both lifetimes.

All [preservation contracts](../contracts.md) apply. No new wire/schema, dependency,
compatibility wrapper, timing policy or gameplay decision is introduced.

## Deletion and cutover

Delete independent manual/sampleRunning/cancelledWork write bundles where the controller or derived operation state replaces them. Keep partial sample evidence and artifact pins.

Migrate consumers in this slice and remove the superseded representation at once.
No compatibility bridge is allowed. If the cutover cannot fit this slice, revise
the slice boundary so its consumers migrate together before implementation.

## Verification and runnable review

Focused existing gates: [mapWorkbenchEditor.test.tsx](../../../web/tests/mapWorkbenchEditor.test.tsx), [mapWorkbenchRunner.test.ts](../../../web/tests/mapWorkbenchRunner.test.ts), [mapWorkbenchSample.test.ts](../../../web/tests/mapWorkbenchSample.test.ts).

Preserve Cancel during sample, cancellation awaiting owned work, rapid edits during manual operation, and no regeneration after explicit cancel. Add missing late-manual completion/edited input cases at the mounted editor/API seam. Do not count callback invocations as the only oracle.

Inspect, compare and sample in the existing workbench; edit during work, cancel, and observe retained results without an unexpected new job.

Follow [verification](../verification.md). If this slice produces a shot, explicitly
run `compare-screenshots` against the frozen before/approved target, then run an
unprimed `screenshot-critique` as the **last visual acceptance check**, and show the
useful evidence with `preview-shots`. The judged crop/variable is: Progress/status and displayed plan during cancellation; no visual redesign.
Everything outside that changed interaction/lifetime stays visually unchanged.
Human feedback uses the README's non-blocking review window.

## Decision budget and exit

Delegated: Private operation kind names and hook/controller placement. Cancellation waiting, resumption policy and partial result retention are fixed.

Feedback that would change scope: New job concurrency or restarting a cancelled sample changes behavior.

Exit only when the focused consumer contracts pass, affected type/lint checks are
green, obsolete ownership is removed, and the substantive-change review is complete.
Record evidence and decisions; update the README handoff before ending the pass.

## Implementation receipt

Baseline: `82a750c55`. `manualOperation.ts` now owns one manual operation record
with kind, controller, captured input/artifact, completion and cancellation
generation. Manual starts await the cancelled predecessor before touching
`LatestRunner`; queued cancel/edit and late completion gates are ownership
checked. `LatestRunner`, the protocol, sample collector, server queue and
retention behavior remain unchanged. The old `sampleRunning` and
`cancelledWork` coordination was removed. Partial sample rows and artifact
pins remain retained, and explicit cancel does not resume generation.

Focused evidence: `bun run --cwd web test -- tests/mapWorkbenchEditor.test.tsx tests/mapWorkbenchRunner.test.ts tests/mapWorkbenchSample.test.ts` — 3 files, 23 tests passed. Map API/native/server evidence: the six map-workbench test files — 6 files, 49 tests passed. Typecheck, format check, complexity lint and package lint passed; oxlint retains one pre-existing `unitGhosts.test.ts` `no-explicit-any` warning. Mutation evidence is in
`throwaway/lifecycle-ownership/16-mutation-run.txt`: removing predecessor
completion waiting fails the mounted waiting test (exit 1), then the source is
restored and the focused suite is green.

Shape, diff and docs review lenses are clean; the diff changes only the map
workbench owner and its mounted tests, with no documentation outside this
receipt. Fresh independent Claude Opus high-effort read-only review: CLEAN
(the direct CLI review exposed no shareable thread URL). No visual shot was
produced because layout, styling and map presentation are unchanged.

Production diff: 133 additions and 138 deletions in `MapWorkbench.tsx`, plus 134
new lines in `manualOperation.ts`; test diff: 130 additions and 3 deletions in
`mapWorkbenchEditor.test.tsx`.
