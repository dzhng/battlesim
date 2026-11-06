# 07 — Purchase choice, preview and confirmation

**Status:** pending. **Audit families:** 19. **Dependencies:** 01, 06.

## Contract and question

Can purchase state distinguish current intent from outstanding preview and confirmation work?

Current owners/consumers: [purchasePlacement.ts](../../../web/src/battle/input/purchasePlacement.ts), [useBattleSession.ts](../../../apps/battle-lab/src/useBattleSession.ts).

## API seam and target shape

Retain PurchasePlacementControl and its choose/cancel/at/confirm interface. Replace correlated flags with a choice record, active preview capture, retained answer certificate/relative soldier shape and active confirmation capture. Optional records, not duplicate busy booleans, say which operations exist. Choice and confirmation may coexist for different generations.

## Transitions and invariants

Choose/cancel revokes choice generation and answer without cancelling already-owned work. at records client/spot/revision and keeps allowed shape; one query runs, then completion admits only matching intent and immediately considers the newest request. confirm captures its ghost and generation; accepted ack returns that ghost and clears only a still-matching choice. Rejection/absence clears confirmation ownership in finally. Preserve stale-knowledge tolerance at the same spot.

All [preservation contracts](../contracts.md) apply. No new wire/schema, dependency,
compatibility wrapper, timing policy or gameplay decision is introduced.

## Deletion and cutover

Delete inFlight/confirming and duplicated validity state only where operation records replace them. Do not delete independent client, spot or knowledge identity. No shared scheduler with move previews.

Migrate consumers in this slice and remove the superseded representation at once.
No compatibility bridge is allowed. If the cutover cannot fit this slice, revise
the slice boundary so its consumers migrate together before implementation.

## Verification and runnable review

Focused existing gates: [purchasePlacement.test.ts](../../../web/tests/purchasePlacement.test.ts), [skirmishPurchases.test.ts](../../../web/tests/skirmishPurchases.test.ts), [unitControl.test.tsx](../../../web/tests/unitControl.test.tsx).

Keep current variant/destination/client invalidation, confirm without pointer movement, retained squad shape, blocked-ground snapping and rapidly changing knowledge cases. Add cancellation/new choice during query and confirmation, including old accepted ack after new choice, through the public controller.

In the existing battle purchase interaction, choose a unit, move/rotate its ghost, confirm, cancel and choose again while replies are delayed.

Follow [verification](../verification.md). If this slice produces a shot, explicitly
run `compare-screenshots` against the frozen before/approved target, then run an
unprimed `screenshot-critique` as the **last visual acceptance check**, and show the
useful evidence with `preview-shots`. The judged crop/variable is: Purchase ghost, cursor and placement feedback at fixed camera/tick; no visual tuning.
Everything outside that changed interaction/lifetime stays visually unchanged.
Human feedback uses the README's non-blocking review window.

## Decision budget and exit

Delegated: Names of private capture/certificate records. The one-query cadence and confirmation identity rules are fixed.

Feedback that would change scope: Changing what Cancel does to an already issued purchase or demanding the newest knowledge revision would change behavior.

Exit only when the focused consumer contracts pass, affected type/lint checks are
green, obsolete ownership is removed, and the substantive-change review is complete.
Record evidence and decisions; update the README handoff before ending the pass.
