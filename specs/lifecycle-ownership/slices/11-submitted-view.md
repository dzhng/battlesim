# 11 — Submitted view snapshot

**Status:** complete. **Audit families:** 27. **Dependencies:** 10.

## Contract and question

Can every pick, camera ray and frame waiter refer to the same successfully drawn scene generation?

Current owners/consumers: [LabViewport.tsx](../../../apps/battle-lab/src/LabViewport.tsx), [viewportFrames.ts](../../../apps/battle-lab/src/viewportFrames.ts), [viewportCamera.ts](../../../packages/renderer-core/src/viewportCamera.ts).

## API seam and target shape

Group submitted camera/projection, pick boxes, observation and owning scene identity in one local submitted-view snapshot. Keep ViewportFrames as the existing waiter owner; the scene-admission owner supplies the current scene, not a second submitted snapshot. Camera intent and current prepared frame remain separate from submitted evidence.

## Transitions and invariants

Prepare/attempt draw without advancing picks. On successful render, atomically publish the snapshot and resolve eligible frame waiters in the existing order. A skipped surface retains prior rays/picks/observation and keeps requests pending. Replacement cannot expose unsubmitted art or a retired observer. Keep readiness callbacks and onFrame placement timing unchanged.

All [preservation contracts](../contracts.md) apply. No new wire/schema, dependency,
compatibility wrapper, timing policy or gameplay decision is introduced.

## Deletion and cutover

Delete independently advanced submittedView/submittedPicks and equivalent duplicated submitted facts once the snapshot owns them; preserve local scratch geometry.

Migrate consumers in this slice and remove the superseded representation at once.
No compatibility bridge is allowed. If the cutover cannot fit this slice, revise
the slice boundary so its consumers migrate together before implementation.

## Verification and runnable review

Focused existing gates: [viewport.test.tsx](../../../web/tests/viewport.test.tsx), [picking.test.ts](../../../web/tests/picking.test.ts), [camera3d.test.ts](../../../web/tests/camera3d.test.ts).

Preserve the named viewport cases for skipped surface, submitted pick boxes/rays, desired art and replacement frame waiters. Assert actual projected/picked values and completion, not snapshot structure. Use a real browser input scene only for GPU submission/DOM alignment questions.

Pause, resize or replace the scene, skip a surface via the existing test seam, and confirm input addresses what was actually drawn.

Follow [verification](../verification.md). If this slice produces a shot, explicitly
run `compare-screenshots` against the frozen before/approved target, then run an
unprimed `screenshot-critique` as the **last visual acceptance check**, and show the
useful evidence with `preview-shots`. The judged crop/variable is: Picked object and matching cursor/selection plus whole viewport; matched camera and observer.
Everything outside that changed interaction/lifetime stays visually unchanged.
Human feedback uses the README's non-blocking review window.

## Decision budget and exit

Delegated: Snapshot name and private publication helper. Successful-submit gating and observer ownership are fixed.

Feedback that would change scope: Picking against newest simulation data rather than the drawn frame is an explicitly forbidden behavior change.

Exit only when the focused consumer contracts pass, affected type/lint checks are
green, obsolete ownership is removed, and the substantive-change review is complete.
Record evidence and decisions; update the README handoff before ending the pass.

## Implementation receipt

Baseline: `a07756197`. `ViewportSubmittedView` now owns the last successful
scene/camera/pick snapshot. `ViewportFrames` remains the sole waiter owner;
publication still occurs only after `draw()` succeeds, in the existing callback
and waiter order. Rays and projection retain the last drawn camera across scene
replacement, while picks are withheld until the installed scene submits. Frame
picks carry over only within the same scene; route picks and proxy picks are
re-read on each successful submission. Retired staged callbacks are discarded
when `SceneAdmission` installs a replacement. The session observation remains
owned by the existing `submitted` callback rather than retaining a second
presentation-module view in the snapshot.

Focused checks:

- `bun run --cwd web test -- tests/viewport.test.tsx tests/viewportReplacement.test.tsx tests/picking.test.ts tests/camera3d.test.ts` — 47 tests passed.
- `bun run --cwd web typecheck` — passed.
- `bun run --cwd web lint` — passed with the existing `unitGhosts.test.ts:150` `no-explicit-any` warning.
- Changed-file ESLint, repository `fmt:check`, and `git diff --check` — passed.
- `web/verify` was attempted; scene runs were blocked by unfetched Git LFS runtime art/baselines and existing audio decode failures. No visual shot was produced, so screenshot comparison/critique/Preview were not applicable.

Mutation evidence: `throwaway/lifecycle-ownership/mutation-skipped-submit.log` fails when skipped rendering is changed to publish/complete; `throwaway/lifecycle-ownership/mutation-stale-pick.log` fails when replacement scene identity gating is removed. Both mutants were restored and the focused suite passed.

Fresh independent Claude Opus high-effort read-only review: [receipt](../../../throwaway/lifecycle-ownership/claude-review-final.txt) — code and scope clean. The review's documentation and code tidies were applied before this receipt was finalized.

Production diff: 80 added, 12 deleted lines across `LabViewport.tsx` and
`viewportSubmittedView.ts`; new `viewportSubmittedView.ts` is 64 lines. Test
diff: 124 added, 35 deleted lines across `viewport.test.tsx` and the new
`viewportReplacement.test.tsx`; both remain under the source-size ratchet.
