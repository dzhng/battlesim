# 01 — Simulation client request ownership

**Status:** pending. **Audit families:** 9. **Dependencies:** none.

## Contract and question

Can each pending operation own the information needed to complete it, without changing request settlement or the wire?

Current owners/consumers: [client.ts](../../../web/src/battle/sim/client.ts), [protocol.ts](../../../web/src/battle/sim/protocol.ts).

## API seam and target shape

Keep SimClient and SimReply unchanged. Inside createSimClient, use separate typed families for commands, previews, advances and the existing single replay waiter. An advance entry owns its resolver/rejecter and optional acknowledged target. A local client lifetime distinguishes active, failed and disposed; resource cleanup obligations must survive failure. Keep credit ownership in the existing buffer/Publication path.

## Transitions and invariants

Request registers in its family before send; reply settles/removes the matching entry. Advance records its target, then checks releasedTick, and release checks the same entries. Centralize failure/disposal policy with the precise per-family outcomes in contracts.md. Preserve query-only failure, sequence namespaces and status ordering. Do not settle additional promises on disposal or invent concurrent replay semantics.

All [preservation contracts](../contracts.md) apply. No new wire/schema, dependency,
compatibility wrapper, timing policy or gameplay decision is introduced.

## Deletion and cutover

Delete advanceTargets and its separate cleanup; remove duplicate terminal mutation policy after callers use the owner. Do not merge unrelated numeric ID spaces into one untyped registry.

Migrate consumers in this slice and remove the superseded representation at once.
No compatibility bridge is allowed. If the cutover cannot fit this slice, revise
the slice boundary so its consumers migrate together before implementation.

## Verification and runnable review

Focused existing gates: [client.test.ts](../../../web/tests/client.test.ts), [authority.test.ts](../../../web/tests/authority.test.ts), [purchasePlacement.test.ts](../../../web/tests/purchasePlacement.test.ts), [simSession.test.tsx](../../../web/tests/simSession.test.tsx).

Characterize disposal before structural edits: pending previews reject; other already-pending operations are not newly settled. Use bounded controlled message/microtask turns. Exercise advance acknowledgement before and after final publication release, malformed publication and duplicate release. Existing raw-NaN-carrier and replay-digest cases remain exact.

The existing authority lab can command, pause, advance and withhold/return credit. This is the first useful playable checkpoint; an additional browser run is needed only for an actual worker-path question.

Follow [verification](../verification.md). If this slice produces a shot, explicitly
run `compare-screenshots` against the frozen before/approved target, then run an
unprimed `screenshot-critique` as the **last visual acceptance check**, and show the
useful evidence with `preview-shots`. The judged crop/variable is: Authority controls and acknowledgement log if captured; no visual styling change.
Everything outside that changed interaction/lifetime stays visually unchanged.
Human feedback uses the README's non-blocking review window.

## Decision budget and exit

Delegated: Private family-record names and helper placement. The client stays the domain owner; no RPC package or event interpreter.

Feedback that would change scope: A request to support concurrent replay snapshots or uniform rejection on disposal changes the contract and requires a separate correction decision.

Exit only when the focused consumer contracts pass, affected type/lint checks are
green, obsolete ownership is removed, and the substantive-change review is complete.
Record evidence and decisions; update the README handoff before ending the pass.
