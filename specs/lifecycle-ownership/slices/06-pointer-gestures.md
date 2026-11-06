# 06 — Viewport pointer gesture owner

**Status:** complete. **Audit families:** 16. **Dependencies:** none.

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

## Implementation receipt

Baseline: `125553435`. The viewport now delegates left press, right press and
middle-button orbit transitions to `ViewportGestureOwner` in
`apps/battle-lab/src/viewportInput.ts`; `LabViewport` retains DOM eligibility,
pointer capture, listener lifetime, hover and modifiers. Press-time right picks,
release-time facing rays, left click slop, placement drags, selection boxes,
HUD ownership, normal-release hover and blur/Escape/pointercancel cancellation
remain unchanged. The old `press`, `rightPress`, `orbit` records and duplicated
cancellation writes were deleted.

Focused evidence:

- `bun run --cwd web test -- tests/viewport.test.tsx tests/unitControl.test.tsx tests/selectSimilar.test.ts tests/heldKeys.test.ts` — 74 tests passed.
- `bun run --cwd web typecheck` — passed after `bun run --cwd web build:wasm` generated the local WASM modules.
- Affected Oxlint and ESLint checks — passed; Oxlint retained one pre-existing exhaustive-deps warning at `LabViewport.tsx:583`.
- `oxfmt --check` and `git diff --check` — passed.
- Mutation receipts: `throwaway/lifecycle-ownership/mutation-left-release.log` fails when the inclusive 5px release slop becomes exclusive; `mutation-release-ray.log` fails when the release ray is removed.

Claude Opus high-effort review: <t3-thread://v2/9950c6b4-6db9-4544-9904-e8f629486d1f/thread:delegated-task:command%3Amcp%3Ab3b517b7-a383-4bea-93db-2809f949c8c0%3Adelegate-task%3A4f878512-9e96-490f-8932-abbec08273f7>. The review found a click-slop regression, an orbit-release regression and test gaps. Those were fixed by restoring release-distance slop semantics, clearing orbit on any pointerup as before, moving preservation cases into `viewport.test.tsx`, and adding blur/Escape/pointercancel, exact-slop and left-action coverage. A remaining note that real browser button chords arrive through pointer movement describes an existing adapter limitation and caused no behavior change in this preservation slice.

No screenshot was produced: this changes event ownership and lifetime only; the
player-facing geometry and HUD are unchanged.
