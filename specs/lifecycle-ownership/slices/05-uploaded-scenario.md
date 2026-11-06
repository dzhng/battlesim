# 05 — Uploaded scenario route state

**Status:** complete. **Audit families:** 13. **Dependencies:** 02, 04.

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

## Implementation receipt

The route reducer now carries uploaded package/controller identity through the ready
phase. The preparation effect remains active across deploying→ready and cancels on
route cleanup/replacement, so the adopted worker has a continuous owner. Upload-start
and late-read policies preserve the old visible ordering; ready-phase errors retain
the existing invisible error behavior.

Mounted route coverage in `web/tests/scenarioPlay.test.tsx` proves upload → controller
choice → Deploy, waits for session readiness/catalog admission, and verifies the
adopted preparation is not cancelled. The cancellation mutation at
`throwaway/lifecycle-ownership/05-mutation.txt` fails when the handoff guard is
removed. Focused route/scenario/router/preparation checks pass 19 tests; typecheck,
ESLint and Oxlint pass. Claude final review and re-review: <t3-thread://v2/9950c6b4-6db9-4544-9904-e8f629486d1f/thread:delegated-task:command%3Amcp%3Acdbb94b3-c200-4d34-a874-96e9d1cc0c87%3Adelegate-task%3Ad5c0978f-3d6b-497e-b48b-cd8ad3eaf3b7>. Its findings were fixed: stable error dispatch and a test that waits for the actual ready-only catalog effect.

No public route, package format, controller meaning, worker protocol or gameplay
behavior changed. Production route diff: 135 additions, 37 deletions; test addition
provides the mounted boundary proof.


Follow-up Claude review: <t3-thread://v2/9950c6b4-6db9-4544-9904-e8f629486d1f/thread:delegated-task:command%3Amcp%3Acdbb94b3-c200-4d34-a874-96e9d1cc0c87%3Adelegate-task%3A3d83d06f-7975-42c8-bd4e-e051832323a8>. It found and we fixed the effect dependency warning by deriving the preparation effect solely from the non-null package/controller identity; this prevents an accidental dependency on the deploying projection from cancelling handoff. The mounted test now verifies both handoff survival and route-unmount cleanup.
