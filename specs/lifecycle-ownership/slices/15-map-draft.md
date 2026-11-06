# 15 — Map draft and displayed artifact state

**Status:** pending. **Audit families:** 76. **Dependencies:** none.

## Contract and question

Can latest requested input and last admitted displayed output have distinct, explicit ownership?

Current owners/consumers: [MapWorkbench.tsx](../../../apps/map-workbench/src/MapWorkbench.tsx), [runner.ts](../../../apps/map-workbench/src/runner.ts).

## API seam and target shape

Add a map-local pure editor reducer for snapshot/draft/unfinished input, latest request/result, accepted displayed map, saved baseline and artifact-scoped inspection/selection. Keep LatestRunner as the current bounded debounce/coalescing scheduler. Request completion carries the captured input and artifact identity; view selectors derive current/stale and shown output. Controls such as search, zoom and layer visibility remain independent UI state.

## Transitions and invariants

Valid edits schedule latest draft; unfinished input invalidates generation; success updates latest and admitted output, refusal updates latest without erasing admitted geometry. Saved comparison persists through permitted draft edits and resets on seed/snapshot changes. Inspection and selection match the displayed artifact identity, even when feature IDs repeat.

All [preservation contracts](../contracts.md) apply. No new wire/schema, dependency,
compatibility wrapper, timing policy or gameplay decision is introduced.

## Deletion and cutover

Delete scattered setLatest/setAccepted/baseline/review-reset coordination and duplicated current-input interpretation from JSX. Do not replace LatestRunner or duplicate server admission policy.

Migrate consumers in this slice and remove the superseded representation at once.
No compatibility bridge is allowed. If the cutover cannot fit this slice, revise
the slice boundary so its consumers migrate together before implementation.

## Verification and runnable review

Focused existing gates: [mapWorkbenchEditor.test.tsx](../../../web/tests/mapWorkbenchEditor.test.tsx), [mapWorkbenchRunner.test.ts](../../../web/tests/mapWorkbenchRunner.test.ts), [mapWorkbenchSample.test.ts](../../../web/tests/mapWorkbenchSample.test.ts).

Preserve accepted → refused edit, superseded success, baseline reuse/retention and artifact-scoped selection cases. Assert displayed actual artifact/report data and input, not reducer tags. Preserve unfinished text reapply/undo behavior.

Existing map workbench: generate admitted draft, make refused edit, inspect retained plan, compare saved defaults and change seed.

Follow [verification](../verification.md). If this slice produces a shot, explicitly
run `compare-screenshots` against the frozen before/approved target, then run an
unprimed `screenshot-critique` as the **last visual acceptance check**, and show the
useful evidence with `preview-shots`. The judged crop/variable is: Plan, selection and status/review controls; map inputs and visualization styling frozen.
Everything outside that changed interaction/lifetime stays visually unchanged.
Human feedback uses the README's non-blocking review window.

## Decision budget and exit

Delegated: Private state/event names and selectors. Accepted/latest/baseline retention and request identity are fixed.

Feedback that would change scope: Always showing the newest result instead of last admitted geometry is not a cleanup.

Exit only when the focused consumer contracts pass, affected type/lint checks are
green, obsolete ownership is removed, and the substantive-change review is complete.
Record evidence and decisions; update the README handoff before ending the pass.
