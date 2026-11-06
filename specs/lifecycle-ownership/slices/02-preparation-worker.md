# 02 — One preparation worker handoff

**Status:** complete. **Audit families:** 2. **Dependencies:** none.

## Contract and question

Can ordinary and uploaded preparation share worker ownership while retaining their distinct result admission?

Current owners/consumers: [client.ts](../../../web/src/battle/prepare/client.ts), [worker.ts](../../../web/src/battle/prepare/worker.ts).

## API seam and target shape

Keep prepareBattle, prepareUploadedScenarioBundle, Preparation and SimConnection public shapes. A preparation-local owner owns the Worker, cancellation authority and installed handlers. The existing promise carries preparation settlement; replacing the handlers carries connection handoff. Do not add a phase enum that merely duplicates those facts. The two entry points supply their existing success/refusal interpretation; keep this shared helper in the preparation boundary, not a cross-app lifecycle library.

## Transitions and invariants

Start worker and receive stages; accepted result exposes connection adoption over that same worker; connect replaces preparation handlers with simulation handlers. Cancel/close terminates and silences late messages. A pending cancelled preparation remains unsettled. Refusal/error retains its diagnostics, console reporting and termination. Preserve current repeated-call behavior; do not add new refusal semantics merely to enforce an internal type.

All [preservation contracts](../contracts.md) apply. No new wire/schema, dependency,
compatibility wrapper, timing policy or gameplay decision is introduced.

## Deletion and cutover

Delete duplicate connection, handler replacement and worker termination bodies from the two preparation functions. Both consumers cut over together.

Migrate consumers in this slice and remove the superseded representation at once.
No compatibility bridge is allowed. If the cutover cannot fit this slice, revise
the slice boundary so its consumers migrate together before implementation.

## Verification and runnable review

Focused existing gates: [preparationClient.test.ts](../../../web/tests/preparationClient.test.ts), [preparationWorker.test.ts](../../../web/tests/preparationWorker.test.ts), [prepareBattle.test.ts](../../../web/tests/prepareBattle.test.ts), [scenarioBundle.test.ts](../../../web/tests/scenarioBundle.test.ts).

Extend ordinary-path cancellation/handoff coverage to bundle preparation at the same fake worker seam before changing code. Check cancellation before success, after a queued reply, and after connection adoption. Reuse the worker replay test for one authority rather than constructing a second simulation.

Deploy through the lobby and upload an existing canonical scenario package. Cancel preparation and navigate away without a late battle appearing.

Follow [verification](../verification.md). If this slice produces a shot, explicitly
run `compare-screenshots` against the frozen before/approved target, then run an
unprimed `screenshot-critique` as the **last visual acceptance check**, and show the
useful evidence with `preview-shots`. The judged crop/variable is: Loading/refusal plate and first battlefield frame, with the same package or generation request.
Everything outside that changed interaction/lifetime stays visually unchanged.
Human feedback uses the README's non-blocking review window.

## Decision budget and exit

Delegated: Private worker-owner name and the narrow typed decoder callback. Request/reply vocabulary and cancellation settlement are fixed.

Feedback that would change scope: Different cancellation feedback or ability to reconnect an adopted worker would require reslicing.

Exit only when the focused consumer contracts pass, affected type/lint checks are
green, obsolete ownership is removed, and the substantive-change review is complete.
Record evidence and decisions; update the README handoff before ending the pass.

## Implementation receipt

Baseline: `32e605e`. Ordinary/bundle cancellation and handoff passed before edits.
Candidate preparation client, worker, battle and scenario-bundle suites passed
13 cases; typecheck, ESLint and Oxlint passed. Mutation evidence at
`throwaway/lifecycle-ownership/02-mutation.txt` proves cancellation assertions fail
when the cancellation latch is disabled, followed by a restored green run.
Claude review: <t3-thread://v2/9950c6b4-6db9-4544-9904-e8f629486d1f/thread:delegated-task:command%3Amcp%3Acdbb94b3-c200-4d34-a874-96e9d1cc0c87%3Adelegate-task%3A0daa0e8f-6fb2-4b24-9605-d72cc794e99a>.
Verdict clean. Accepted the shared private worker owner and explicit ordinary/bundle
admission callbacks. The review's mutation-receipt note was resolved by running a
latch-only mutation separately at `throwaway/lifecycle-ownership/02-mutation-queued.txt`;
the queued-reply assertion fails and the restored focused suite passes. The
pre-existing bundle stage type mismatch is outside this cleanup and is recorded as
separate source-derived behavior debt, not silently corrected. The boolean logging
option remains private and behavior-preserving; no public or numeric contract
changed.

Shape/diff/docs review: duplicated worker handlers, adoption and termination are
removed; no compatibility path or new public type was introduced. Existing worker
and route docs remain accurate. Candidate visual captures were compared against the
baseline with `visual-parity-diff`; all four pairs are pixel-identical within the
existing capture noise (worst distance 0.00002). The unchanged visual surface needs
no screenshot critique beyond the required preservation record. Production diff:
74 additions, 107 deletions; tests/docs carry the remaining coverage and receipt.
