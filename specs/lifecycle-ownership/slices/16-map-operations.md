# 16 — Map manual operation lifetime

**Status:** pending. **Audit families:** 76. **Dependencies:** 15.

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
