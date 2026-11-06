# 04 — Session startup and warm-up ownership

**Status:** pending. **Audit families:** 6. **Dependencies:** 01, 02, 03.

## Contract and question

Can every startup continuation be tied to one session without coupling authority readiness to viewport readiness?

Current owners/consumers: [useSimSession.ts](../../../apps/battle-lab/src/useSimSession.ts), [BattleView.tsx](../../../apps/battle-lab/src/BattleView.tsx), [ScenePlayer.tsx](../../../apps/battle-lab/src/capture/ScenePlayer.tsx).

## API seam and target shape

Use a session-local startup operation carrying the current client, scripted plan and progression through ready, warm advancement, optional hold and real-time start. Keep authority-ready and viewport-ready as separate facts. useSimSession remains the React adapter for current observation/status/refusals and the stable latest/ground holders; do not put full observation arrays into a new React reducer.

## Transitions and invariants

Create/restart captures scenario and opening side; only initial non-replay generation adopts a prepared connection. Ready and viewport notifications both consult the current operation. Script advance reaches warm tick, optional hold settles, then existing resume/onWarm ordering applies. Cleanup invalidates that operation before disposing its client and cancelling pending RAF. Retain every-publication decode and once-per-RAF React projection.

All [preservation contracts](../contracts.md) apply. No new wire/schema, dependency,
compatibility wrapper, timing policy or gameplay decision is introduced.

## Deletion and cutover

Delete loose warm/plan/ready continuation coordination when the startup operation owns it; remove repeated initialization/reset bundles without changing their order or retained telemetry.

Migrate consumers in this slice and remove the superseded representation at once.
No compatibility bridge is allowed. If the cutover cannot fit this slice, revise
the slice boundary so its consumers migrate together before implementation.

## Verification and runnable review

Focused existing gates: [simSession.test.tsx](../../../web/tests/simSession.test.tsx), [battleStart.test.tsx](../../../web/tests/battleStart.test.tsx), [battleVisit.test.tsx](../../../web/tests/battleVisit.test.tsx), [labLoading.test.tsx](../../../web/tests/labLoading.test.tsx), [capture.test.ts](../../../web/tests/capture.test.ts).

Prove restart uses captured scenario, stale ready/hold completion cannot act on a replacement, and all publications decode while React displays the newest once. If a stale completion currently changes output, name and isolate that correction before changing it; do not silently combine it with extraction.

Deploy and Restart a battle; visit the menu reel and a scripted/capture route. The first playable frame and first reel framing remain the same.

Follow [verification](../verification.md). If this slice produces a shot, explicitly
run `compare-screenshots` against the frozen before/approved target, then run an
unprimed `screenshot-critique` as the **last visual acceptance check**, and show the
useful evidence with `preview-shots`. The judged crop/variable is: Loading-to-first-frame transition and warm reel framing; fixed camera and tick.
Everything outside that changed interaction/lifetime stays visually unchanged.
Human feedback uses the README's non-blocking review window.

## Decision budget and exit

Delegated: Name/location of the private startup controller. Keep feed/RAF boundaries, ready gates and scripted callback ordering.

Feedback that would change scope: Different loading semantics, capture start timing or auto-resume policy requires an explicit contract change.

Exit only when the focused consumer contracts pass, affected type/lint checks are
green, obsolete ownership is removed, and the substantive-change review is complete.
Record evidence and decisions; update the README handoff before ending the pass.
