# 08 — Order preview admission separated from paint

**Status:** pending. **Audit families:** 20. **Dependencies:** 01, 06.

## Contract and question

Can move/building preview own async answers while PointerPaint only builds the displayed geometry?

Current owners/consumers: [pointerPaint.ts](../../../apps/battle-lab/src/pointerPaint.ts), [useBattleSession.ts](../../../apps/battle-lab/src/useBattleSession.ts).

## API seam and target shape

Create a local OrderPreview owner beside pointer paint. update accepts the existing PointerPreview, selected-unit inputs, client, gesture/eligibility identity and knowledge revision; its snapshot carries destinations, building answer and derived availability. PointerPaint remains the mesh/feed owner and consumes that snapshot. The preview owner carries current gesture, active request and last answer; no generic request framework.

## Transitions and invariants

Gesture/client change revokes old answers. At most one request runs; further queries begin on the existing next-frame update cadence. Facing-only changes retain last destinations where currently allowed. Successful answer derives ready/pending/blocked from its actual placement; query failure remains unproven/pending, never fabricated blocked. Keep no-query idle behavior.

All [preservation contracts](../contracts.md) apply. No new wire/schema, dependency,
compatibility wrapper, timing policy or gameplay decision is introduced.

## Deletion and cutover

Delete async request ownership from PointerPaint and remove duplicated preview status fields in consumers. Keep mesh-cache identity and geometric helpers where they belong.

Migrate consumers in this slice and remove the superseded representation at once.
No compatibility bridge is allowed. If the cutover cannot fit this slice, revise
the slice boundary so its consumers migrate together before implementation.

## Verification and runnable review

Focused existing gates: [unitControl.test.tsx](../../../web/tests/unitControl.test.tsx), [blockedTrajectory.test.tsx](../../../web/tests/blockedTrajectory.test.tsx), [rejectedOrder.test.tsx](../../../web/tests/rejectedOrder.test.tsx), [viewport.test.tsx](../../../web/tests/viewport.test.tsx).

Find and extend existing delayed-preview consumer cases. Add a focused integration test at the production controller boundary if needed for A superseded by B, facing updates, failed query and changed client. Do not compare private state labels; assert admitted destinations and cursor/dispatch consequences.

Existing cursor, movement and garrison interaction paths show the same markers for pending, admitted and unavailable actions.

Follow [verification](../verification.md). If this slice produces a shot, explicitly
run `compare-screenshots` against the frozen before/approved target, then run an
unprimed `screenshot-critique` as the **last visual acceptance check**, and show the
useful evidence with `preview-shots`. The judged crop/variable is: Destination/facing markers and building-action cursor; matched paused frame and selection.
Everything outside that changed interaction/lifetime stays visually unchanged.
Human feedback uses the README's non-blocking review window.

## Decision budget and exit

Delegated: Private owner file/type names and projection accessors. Query cadence, result freshness and availability semantics are fixed.

Feedback that would change scope: New speculative availability or clearing a retained shape during every query would require a behavior decision.

Exit only when the focused consumer contracts pass, affected type/lint checks are
green, obsolete ownership is removed, and the substantive-change review is complete.
Record evidence and decisions; update the README handoff before ending the pass.
