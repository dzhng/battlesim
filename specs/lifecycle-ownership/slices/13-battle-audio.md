# 13 — Battle audio graph lifetime

**Status:** complete. **Audit families:** 69. **Dependencies:** 12.

## Contract and question

Can one disposable battle graph carry preparation/readiness/error state and reject stale admission by identity?

Current owners/consumers: [battleAudio.ts](../../../packages/battle-audio/src/battleAudio.ts), [appAudio.ts](../../../packages/battle-audio/src/appAudio.ts).

## API seam and target shape

Keep BattleAudio. A local graph record owns sink, frame and voices plus its preparing/ready/failed state; disconnected/disposed lifetime is explicit. Speaker mappings, identified enemy kinds and shout-pool ownership retain their existing scope. Graph identity, not merely a ready/disposed boolean, guards async completion.

## Transitions and invariants

Connect once to page context/bank; prepare only required battle sounds; admit ready only into the same live graph; then reset old evidence and start voice loading in the current order. Before-ready/muted publications are dropped. Disposal clears battle evidence/mixer and releases the page's battle reference while shared bank preparation survives for another battle.

All [preservation contracts](../contracts.md) apply. No new wire/schema, dependency,
compatibility wrapper, timing policy or gameplay decision is introduced.

## Deletion and cutover

Delete the sink/frame/voices nullable triple plus separate ready/error coordination after graph ownership replaces it. Do not rewrite SoundBank, Chatter, Rust sound decisions or voice caching.

Migrate consumers in this slice and remove the superseded representation at once.
No compatibility bridge is allowed. If the cutover cannot fit this slice, revise
the slice boundary so its consumers migrate together before implementation.

## Verification and runnable review

Focused existing gates: [battleAudio.test.ts](../../../web/tests/battleAudio.test.ts), [appAudio.test.ts](../../../web/tests/appAudio.test.ts), [soundFrame.test.ts](../../../web/tests/soundFrame.test.ts).

Preserve pre-ready drop, preparation failure, departure during preparation, shared pending bank across replacement and enemy voice-set admission. Test a late old graph completion with a current new graph. Compare actual admitted/dropped commands, not internal tags.

A battle begins sounding only once ready; leaving and revisiting cannot replay old combat or retain its mixer tail.

Follow [verification](../verification.md). If this slice produces a shot, explicitly
run `compare-screenshots` against the frozen before/approved target, then run an
unprimed `screenshot-critique` as the **last visual acceptance check**, and show the
useful evidence with `preview-shots`. The judged crop/variable is: Audio/loading controls if captured; no UI or sound-content tuning.
Everything outside that changed interaction/lifetime stays visually unchanged.
Human feedback uses the README's non-blocking review window.

## Decision budget and exit

Delegated: Private graph type and local state accessor. Shared-bank lifetime and evidence admission/reset rules are fixed.

Feedback that would change scope: Buffering pre-ready/muted combat to play later is explicitly outside this contract.

Exit only when the focused consumer contracts pass, affected type/lint checks are
green, obsolete ownership is removed, and the substantive-change review is complete.
Record evidence and decisions; update the README handoff before ending the pass.

## Implementation receipt

The candidate passes `battleAudio`, `appAudio` and `soundFrame` suites: 36 tests;
typecheck, ESLint and Oxlint also pass. Claude final review: <t3-thread://v2/9950c6b4-6db9-4544-9904-e8f629486d1f/thread:delegated-task:command%3Amcp%3Acdbb94b3-c200-4d34-a874-96e9d1cc0c87%3Adelegate-task%3A68634042-b135-4661-af5b-f22c85fbcd78>.

The private BattleGraph record owns sink, frame, voices, shout pool, preparation
state and error while graph identity guards late async completion. The review found
no reachable behavior drift. Its earlier finding that post-ready preparation errors
must leave an admitted graph live was fixed by setting `failed` only while preparing;
post-ready errors retain the old live/error behavior. Public BattleAudio methods and
BattleAudioStats remain unchanged. Existing tests cover pre-ready drop, failed
preparation, departure/late completion, shared pending bank, voice admission and
disposal. No visual surface changed.
