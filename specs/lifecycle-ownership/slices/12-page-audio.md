# 12 — Page audio resources and handoff

**Status:** complete. **Audit families:** 68. **Dependencies:** none.

## Contract and question

Can page audio separate graph ownership, menu preparation and battle handoff without changing browser permission behavior?

Current owners/consumers: [appAudio.ts](../../../packages/battle-audio/src/appAudio.ts), [AppAudio.tsx](../../../apps/battle-lab/src/AppAudio.tsx).

## API seam and target shape

Keep AppAudio and its public methods/stats. Carry context, bank, menu sink and music player as one successful graph record with terminal cleanup ownership. Menu preparation uses a local unprepared/preparing/ready/failed result. Context-admission failure is distinct from menu-bank failure. Battle identity/score and combatStarted handoff remain separate from graph lifetime; mute reads existing settings.

## Transitions and invariants

Attempt context setup on current triggers; queue resume even if a previous suspend is pending. A context failure blocks admission as today; a menu preparation failure does not disable an otherwise usable battle context. Menu bed survives loading until prepared battle audio updates. Returning to menu/disposal releases correct score/graph; volume changes retain evidence, mute transitions reset it. Preserve stats error precedence from existing writes rather than inventing a new priority.

All [preservation contracts](../contracts.md) apply. No new wire/schema, dependency,
compatibility wrapper, timing policy or gameplay decision is introduced.

## Deletion and cutover

Delete correlated nullable graph initialization and menuReady/menuPreparing combinations after the records own them. Search all consumers before deleting any apparently unused method; do not remove an API based on a draft assertion.

Migrate consumers in this slice and remove the superseded representation at once.
No compatibility bridge is allowed. If the cutover cannot fit this slice, revise
the slice boundary so its consumers migrate together before implementation.

## Verification and runnable review

Focused existing gates: [appAudio.test.ts](../../../web/tests/appAudio.test.ts), [battleMusic.test.ts](../../../web/tests/battleMusic.test.ts), [musicPlayer.test.ts](../../../web/tests/musicPlayer.test.ts).

Keep blocked autoplay, quick mute/unmute, suspended-context recovery, repeated-visit connection baseline and deterministic score seed cases. Add missing menu-bank-failure-with-live-context behavior first. Compare observable playback commands/stats and cleanup; no paid generation or unsolicited audible tests.

Menu → loading → battle → menu, mute/unmute quickly, and retry browser audio permission through a real qualifying gesture when browser evidence is needed.

Follow [verification](../verification.md). If this slice produces a shot, explicitly
run `compare-screenshots` against the frozen before/approved target, then run an
unprimed `screenshot-critique` as the **last visual acceptance check**, and show the
useful evidence with `preview-shots`. The judged crop/variable is: Audio controls only if captured; listening/command evidence owns sound behavior, not screenshots.
Everything outside that changed interaction/lifetime stays visually unchanged.
Human feedback uses the README's non-blocking review window.

## Decision budget and exit

Delegated: Private graph/menu-state names and construction helpers. Context permission, error policy, audio schedule and score identity are fixed.

Feedback that would change scope: Changing autoplay retries, ambience/music handoff or mute evidence policy requires a separate change.

Exit only when the focused consumer contracts pass, affected type/lint checks are
green, obsolete ownership is removed, and the substantive-change review is complete.
Record evidence and decisions; update the README handoff before ending the pass.

## Implementation receipt

Delegated implementation commit: `9a0f1dca0393d790af1ccc0ca28287730a7edc00`,
integrated as `a317e7a4e`. The page audio graph now owns context, bank, menu sink
and music together with terminal cleanup; menu preparation failure remains distinct
from context admission failure. Public AppAudio methods, permission retries, mute
behavior, score identity and stats precedence are unchanged.

Focused app-audio suite: 18 tests passed. Typecheck, Oxlint, formatting and diff
checks passed in the delegated checkout. Claude review: <t3-thread://v2/9950c6b4-6db9-4544-9904-e8f629486d1f/thread:delegated-task:command%3Amcp%3A4a9f2ccf-cbd9-4c5b-8ea2-e57299d3d23c%3Adelegate-task%3A72678dcf-63d8-4627-b244-270c4c587d61>; no blocking findings, cosmetic findings were fixed before integration. No visual surface changed; no screenshot was needed.
