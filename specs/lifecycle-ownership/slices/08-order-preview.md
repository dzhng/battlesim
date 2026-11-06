# 08 — Order preview admission separated from paint

**Status:** complete. **Audit families:** 20. **Dependencies:** 01, 06.

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

## Implementation receipt

`OrderPreview` now owns move/building preview requests, gesture/client generations,
the one-active-query next-frame cadence, retained facing answers, and derived
availability. `PointerPaint` retains the marker geometry, mesh cache and feed;
the session composes its snapshot for cursor, drag ghosts, probe output and paint.
No protocol, public probe shape, visible geometry, timing policy or gameplay rule
changed. The old async fields and `resolvePreview` path were removed.

Focused evidence:

- `bun run --cwd web test -- tests/orderPreview.test.ts tests/unitControl.test.tsx tests/viewport.test.tsx tests/blockedTrajectory.test.tsx tests/rejectedOrder.test.tsx` — 78 tests passed.
- `bun run --cwd web typecheck`, `bun run lint`, and `bun run fmt:check` — passed; only pre-existing exhaustive-deps/type warnings remain.
- Mutation receipts in `throwaway/lifecycle-ownership/`: `order-preview-mutation-generation.log` and `order-preview-mutation-facing.log` both fail when stale invalidation or facing retention is broken.

Shape/diff/docs review is clean. Decision audit found no unsound architectural
choices beyond the slice's delegated private owner naming. No screenshot was
produced: the slice preserves the existing marker/cursor geometry and changes
only async ownership. Final Claude Opus high-effort read-only review: <t3-thread://v2/9950c6b4-6db9-4544-9904-e8f629486d1f/thread:delegated-task:command%3Amcp%3A3f8297f5-ce0b-4fd8-8113-42313cd71b4f%3Adelegate-task%3A0192df0c-3c4a-4ef5-b208-da9232597282>. Verdict: clean; it confirmed admitted gesture/client revocation, one-query cadence, facing retention, failure/idle semantics, and the complete production cutover.
