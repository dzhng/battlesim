# 09 — Released action and acknowledgement ownership

**Status:** pending. **Audit families:** 20. **Dependencies:** 08.

## Contract and question

Can acknowledgement feedback be applied only to the action that was released, independently of the current hover/press?

Current owners/consumers: [useBattleSession.ts](../../../apps/battle-lab/src/useBattleSession.ts), [pointerPaint.ts](../../../apps/battle-lab/src/pointerPaint.ts), [useUnitControl.ts](../../../web/src/battle/input/useUnitControl.ts).

## API seam and target shape

Use one session-local released-action record carrying captured intent/order, queue flag, client and press generation, with the acknowledgement state the current path needs. Keep the live gesture and its preview separate. Existing command issue/ack callbacks remain the boundary; projection to cursor/ghost feedback stays in the current presentation helpers.

## Transitions and invariants

Press captures intended action; release dispatches and retains only its own feedback identity. Ack may correct that captured action, not a later gesture or client. The corresponding publication retires the bridge to observed orders. Reset/new session clears both gesture and released bridge at their existing boundaries. Read the actual writers before replacing fields; preserve all ack/refusal and order-flash timing.

All [preservation contracts](../contracts.md) apply. No new wire/schema, dependency,
compatibility wrapper, timing policy or gameplay decision is introduced.

## Deletion and cutover

Delete coordinated pendingAction/captured/press-generation mutations replaced by the record and remove stale preview/ack adapters. Do not move command authority into the preview owner.

Migrate consumers in this slice and remove the superseded representation at once.
No compatibility bridge is allowed. If the cutover cannot fit this slice, revise
the slice boundary so its consumers migrate together before implementation.

## Verification and runnable review

Focused existing gates: [unitControl.test.tsx](../../../web/tests/unitControl.test.tsx), [rejectedOrder.test.tsx](../../../web/tests/rejectedOrder.test.tsx), [blockedTrajectory.test.tsx](../../../web/tests/blockedTrajectory.test.tsx), [battleIndicators.test.ts](../../../web/tests/battleIndicators.test.ts).

Drive press → release → changed selection/gesture → delayed ack → publication. Assert the newer action remains intact and old feedback retires when observed. Accepted, refused and unproven building cases remain distinct; no class-string assertions.

Issue a move/building order, begin another gesture before acknowledgement, and watch only the released action receive its feedback.

Follow [verification](../verification.md). If this slice produces a shot, explicitly
run `compare-screenshots` against the frozen before/approved target, then run an
unprimed `screenshot-critique` as the **last visual acceptance check**, and show the
useful evidence with `preview-shots`. The judged crop/variable is: Cursor, destination markers and refusal callout during release/ack/publication; frozen look.
Everything outside that changed interaction/lifetime stays visually unchanged.
Human feedback uses the README's non-blocking review window.

## Decision budget and exit

Delegated: Private record name and small helper methods. Client identity, queue flags and bridge retirement policy are fixed.

Feedback that would change scope: Changing optimistic feedback duration or command acceptance handling is outside cleanup.

Exit only when the focused consumer contracts pass, affected type/lint checks are
green, obsolete ownership is removed, and the substantive-change review is complete.
Record evidence and decisions; update the README handoff before ending the pass.
