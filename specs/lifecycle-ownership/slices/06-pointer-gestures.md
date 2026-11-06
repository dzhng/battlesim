# 06 — Viewport pointer gesture owner

**Status:** pending. **Audit families:** 16. **Dependencies:** none.

## Contract and question

Can gesture transitions leave the viewport orchestration without losing multi-button or DOM capture behavior?

Current owners/consumers: [LabViewport.tsx](../../../apps/battle-lab/src/LabViewport.tsx), [viewportInput.ts](../../../apps/battle-lab/src/viewportInput.ts).

## API seam and target shape

Add a viewport-local gesture owner accepting normalized down/move/up/cancel events and emitting the existing pick/box/orbit callbacks. It owns left press, right press and orbit as independent optional records. The browser adapter owns event eligibility, DOM pointer capture and listener lifetime; ray/pick callbacks read the submitted view. Hover position and modifiers stay orthogonal.

## Transitions and invariants

Preserve click slop, press-time right pick, release-time facing ray, left action versus selection-box behavior, HUD native click sequence, camera drag, normal capture release preserving hover, and blur/Escape/cancel clearing gestures. Do not make all buttons one exclusive mode. Input-disabled behavior stays the same.

All [preservation contracts](../contracts.md) apply. No new wire/schema, dependency,
compatibility wrapper, timing policy or gameplay decision is introduced.

## Deletion and cutover

Delete the event block's scattered press/reset mutations and duplicated cancellation sequence after all viewport consumers use the owner. Keep the existing input contract types.

Migrate consumers in this slice and remove the superseded representation at once.
No compatibility bridge is allowed. If the cutover cannot fit this slice, revise
the slice boundary so its consumers migrate together before implementation.

## Verification and runnable review

Focused existing gates: [viewport.test.tsx](../../../web/tests/viewport.test.tsx), [unitControl.test.tsx](../../../web/tests/unitControl.test.tsx), [selectSimilar.test.ts](../../../web/tests/selectSimilar.test.ts), [heldKeys.test.ts](../../../web/tests/heldKeys.test.ts).

Add missing browser-event cases before extraction: overlapping button lanes, lost capture versus normal release, disabled input, blur and HUD press ownership. Existing interaction tests remain the oracle; do not unit-test invented event enum names.

Use existing movement, cursor and transport interaction surfaces: click, box-select, right-drag facing, orbit, Escape and chorded buttons.

Follow [verification](../verification.md). If this slice produces a shot, explicitly
run `compare-screenshots` against the frozen before/approved target, then run an
unprimed `screenshot-critique` as the **last visual acceptance check**, and show the
useful evidence with `preview-shots`. The judged crop/variable is: Pointer/selection/facing region and surrounding HUD; only event ownership may change.
Everything outside that changed interaction/lifetime stays visually unchanged.
Human feedback uses the README's non-blocking review window.

## Decision budget and exit

Delegated: Private module/class name and normalized event structure derived from existing inputs. No alternative gesture recognizer library.

Feedback that would change scope: Exclusive button gestures, new drag thresholds or changed key precedence are outside this slice.

Exit only when the focused consumer contracts pass, affected type/lint checks are
green, obsolete ownership is removed, and the substantive-change review is complete.
Record evidence and decisions; update the README handoff before ending the pass.
