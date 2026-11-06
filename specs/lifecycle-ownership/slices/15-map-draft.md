# 15 — Map draft and displayed artifact state

**Status:** complete. **Audit families:** 76. **Dependencies:** none.

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

## Implementation receipt

Baseline: `c73404e202723508f988bdc48b538b333c450e0f`. `editorState.ts` now owns
the map draft, latest request/result, accepted displayed artifact, saved baseline,
unfinished input and artifact-scoped inspection/selection. `LatestRunner`, the
HTTP protocol and receipt shapes remain unchanged. Refusals update the latest
result while retaining admitted geometry; no-op formatted edits retain draft
identity and do not restart generation. Consumer coverage added the no-op edit
and refused-seed selection cases.

Focused map editor/runner/sample tests pass 18 tests. Web typecheck, web lint,
root format check, complexity lint and root lint pass; existing unrelated lint
warnings remain outside this slice. Mutation evidence is in
`throwaway/lifecycle-ownership/15-mutation-run.txt`: changing refusal handling to
overwrite the accepted artifact fails the retained-plan inspection test (exit 1),
then the source is restored and the focused suite is green.

Shape, diff and docs review lenses are clean; obsolete setter ownership and the
duplicated equality helper are removed. Fresh Claude Opus high-effort read-only
recheck: <t3-thread://v2/9950c6b4-6db9-4544-9904-e8f629486d1f/thread:delegated-task:command%3Amcp%3Adb6e1685-14d4-4dfc-990a-bc0210554029%3Adelegate-task%3Abf6310af-078d-417e-bac1-3ba8a3131f37> — clean after fixing its two behavior findings. Production diff is 266 additions and 106 deletions; tests add 54 lines. No visual shot was produced because styling and layout are unchanged.
