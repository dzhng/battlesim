# 05 — Uploaded scenario route state

**Status:** pending. **Audit families:** 13. **Dependencies:** 02, 04.

## Contract and question

Can each route phase carry the package/session data that belongs to it?

Current owners/consumers: [scenarioPlay.tsx](../../../apps/battle-lab/src/routes/scenarioPlay.tsx), [client.ts](../../../web/src/battle/prepare/client.ts).

## API seam and target shape

Use a route-local discriminated state for empty/importing, controller choice, preparation, playable session and the phase-specific failure payload. Carry the exact uploaded bytes and controller candidates where needed. Keep undefined default controller versus explicit null AI-only choice. Embedded asset playback identity and replay replacement remain separate records, not route phases.

## Transitions and invariants

Upload resets the existing session; read/admit package; choose controller; Deploy starts the existing worker path; readiness installs its complete prepared session. Failure keeps the same retry/retained-package UI as today. Route cleanup silences its owned preparation. Preserve actual file-read completion ordering unless a separately tested correction is recorded.

All [preservation contracts](../contracts.md) apply. No new wire/schema, dependency,
compatibility wrapper, timing policy or gameplay decision is introduced.

## Deletion and cutover

Delete parallel bundle/deploy/session/stage combinations and repeated package-reset bundles once constructors/transitions supply the coherent payload.

Migrate consumers in this slice and remove the superseded representation at once.
No compatibility bridge is allowed. If the cutover cannot fit this slice, revise
the slice boundary so its consumers migrate together before implementation.

## Verification and runnable review

Focused existing gates: [scenarioBundle.test.ts](../../../web/tests/scenarioBundle.test.ts), [scenarioBuilder.test.ts](../../../web/tests/scenarioBuilder.test.ts), [sessionCatalog.test.ts](../../../web/tests/sessionCatalog.test.ts), [preparationClient.test.ts](../../../web/tests/preparationClient.test.ts).

Add a mounted uploaded-route test at the file/worker boundary for upload → controller → Deploy → session and replacement/failure; existing bundle validation alone does not prove route transitions. Keep catalog scope, embedded audio identity and replay controls. Do not add a debug-only route.

The existing scenario player uploads the same canonical package, chooses human or AI-only, deploys and replaces it.

Follow [verification](../verification.md). If this slice produces a shot, explicitly
run `compare-screenshots` against the frozen before/approved target, then run an
unprimed `screenshot-critique` as the **last visual acceptance check**, and show the
useful evidence with `preview-shots`. The judged crop/variable is: Upload/controller/loading plate and first played frame; copy, controls and geometry unchanged.
Everything outside that changed interaction/lifetime stays visually unchanged.
Human feedback uses the README's non-blocking review window.

## Decision budget and exit

Delegated: Private state/event names and small presentation-only subcomponents. Package admission, selected controller meaning and import ordering are fixed.

Feedback that would change scope: New import concurrency, persistence or controller-selection semantics need a separate change.

Exit only when the focused consumer contracts pass, affected type/lint checks are
green, obsolete ownership is removed, and the substantive-change review is complete.
Record evidence and decisions; update the README handoff before ending the pass.
