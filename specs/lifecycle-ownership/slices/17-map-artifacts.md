# 17 — Map service artifact retention

**Status:** pending. **Audit families:** 77. **Dependencies:** 16.

## Contract and question

Can retained map artifacts have one cleanup authority without merging them with save candidates or queued requests?

Current owners/consumers: [server.ts](../../../apps/map-workbench/server.ts), [native.ts](../../../apps/map-workbench/native.ts).

## API seam and target shape

Extract a map-service-local artifact owner from WorkbenchStore. Its narrow operations register a completed result, read a currently valid artifact, retain the caller's admitted displayed IDs, release obsolete storage and close. It owns the artifact map and directory disposal. WorkbenchStore keeps its current bounded request queue, controllers, validation and reviewed-candidate identity; FixturePublication keeps journal/recovery.

## Transitions and invariants

Native completion is admitted or refused using current policy; retain displayed admitted artifacts, preserve the latest refusal receipt, retire only eligible undisplayed geometry, and release sample outputs according to current rules. Cancelled late success cannot release a displayed artifact. Close aborts existing jobs and releases owned artifact storage in current ordering. Keep ID/receipt formats and retention limit.

All [preservation contracts](../contracts.md) apply. No new wire/schema, dependency,
compatibility wrapper, timing policy or gameplay decision is introduced.

## Deletion and cutover

Delete duplicated artifact retirement/storage cleanup from service methods after all consumers use the artifact owner. No new cache hierarchy, garbage collector, janitor timer or changed queue capacity.

Migrate consumers in this slice and remove the superseded representation at once.
No compatibility bridge is allowed. If the cutover cannot fit this slice, revise
the slice boundary so its consumers migrate together before implementation.

## Verification and runnable review

Focused existing gates: [mapWorkbenchServer.test.ts](../../../web/tests/mapWorkbenchServer.test.ts), [mapWorkbenchHttp.test.ts](../../../web/tests/mapWorkbenchHttp.test.ts), [nativeReporter.test.ts](../../../web/tests/nativeReporter.test.ts).

Preserve retained preview-ID validation, late cancelled success, latest refusal receipt, sample-geometry release and close cleanup. Use existing controlled native reporter/temp-directory tests. Fixture transaction tests remain unchanged unless integration touches them.

Existing workbench API driven by its UI retains the displayed map across rejected/cancelled work, while obsolete temporary artifacts are removed.

Follow [verification](../verification.md). If this slice produces a shot, explicitly
run `compare-screenshots` against the frozen before/approved target, then run an
unprimed `screenshot-critique` as the **last visual acceptance check**, and show the
useful evidence with `preview-shots`. The judged crop/variable is: No new visual capture required for the server seam; if captured, displayed retained plan is the frozen target.
Everything outside that changed interaction/lifetime stays visually unchanged.
Human feedback uses the README's non-blocking review window.

## Decision budget and exit

Delegated: Artifact-owner module and method names, with only currently used operations. Admission and cleanup semantics are fixed.

Feedback that would change scope: Longer artifact history, new persistence or different retention counts requires another feature.

Exit only when the focused consumer contracts pass, affected type/lint checks are
green, obsolete ownership is removed, and the substantive-change review is complete.
Record evidence and decisions; update the README handoff before ending the pass.
