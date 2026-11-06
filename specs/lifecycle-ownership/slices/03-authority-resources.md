# 03 — Authority resource lifetime

**Status:** pending. **Audit families:** 7. **Dependencies:** 01.

## Contract and question

Can the authority admit and release its module/battle pair atomically while leaving execution policy readable in pump?

Current owners/consumers: [authority.ts](../../../web/src/battle/sim/authority.ts), [workerAuthority.ts](../../../web/src/battle/sim/workerAuthority.ts).

## API seam and target shape

Keep createAuthority(AuthorityHost), handle and pump. A local lifetime record represents loading, admitted resources {sim,battle}, and closed. Loading backlog and producer credits retain their existing ownership and order. Started, paused, hidden, slow, scripted target and tick timing remain separate execution fields. Do not create a generic reducer/effect engine or replace the explicit pump with a transition table.

## Transitions and invariants

Initialization resolves into one admitted resource pair unless already disposed; replay/scenario failure reports before teardown. Teardown releases the battle once and closes the host. Pump uses the exact precedence in contracts.md. Keep earliest-deadline scheduling and credit-recovery timing. Finished battle remains its authoritative outcome, not a newly stored duplicate lifetime phase.

All [preservation contracts](../contracts.md) apply. No new wire/schema, dependency,
compatibility wrapper, timing policy or gameplay decision is introduced.

## Deletion and cutover

Delete correlated sim/battle nullable writes, disposed/resource contradictions and non-null assertions made unnecessary by the admitted record. Retain all meaningful execution gates.

Migrate consumers in this slice and remove the superseded representation at once.
No compatibility bridge is allowed. If the cutover cannot fit this slice, revise
the slice boundary so its consumers migrate together before implementation.

## Verification and runnable review

Focused existing gates: [authority.test.ts](../../../web/tests/authority.test.ts), [client.test.ts](../../../web/tests/client.test.ts), [preparationWorker.test.ts](../../../web/tests/preparationWorker.test.ts).

Preserve before-start silence, ordered acknowledgements, paused scripted advance, terminal fast-forward, hidden/pause precedence, load/dispose race, debt shedding and credit stall. Use the injected host clock and module boundary. Preserve raw publication words and accepted-command replay digests.

Existing authority lab: pause, step, restart and credit stall still produce the same ticks and acknowledgements.

Follow [verification](../verification.md). If this slice produces a shot, explicitly
run `compare-screenshots` against the frozen before/approved target, then run an
unprimed `screenshot-critique` as the **last visual acceptance check**, and show the
useful evidence with `preview-shots`. The judged crop/variable is: Authority controls and status/log; visible output is frozen.
Everything outside that changed interaction/lifetime stays visually unchanged.
Human feedback uses the README's non-blocking review window.

## Decision budget and exit

Delegated: Private lifetime record names and local access helpers only. Scheduling, status precedence and tick work are not delegated.

Feedback that would change scope: Changing whether scripted advancement honors visibility/pause or how debt is repaid is outside cleanup.

Exit only when the focused consumer contracts pass, affected type/lint checks are
green, obsolete ownership is removed, and the substantive-change review is complete.
Record evidence and decisions; update the README handoff before ending the pass.
