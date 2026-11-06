# 19 — Audition operation and resource ownership

**Status:** pending. **Audit families:** 79. **Dependencies:** 18.

## Contract and question

Can one audition run own loading/playback/stop so UI fields cannot describe different runs?

Current owners/consumers: [audition.ts](../../../apps/sound-workbench/src/audition.ts), [SoundWorkbench.tsx](../../../apps/sound-workbench/src/SoundWorkbench.tsx), [ChatterPreview.tsx](../../../apps/sound-workbench/src/voices/ChatterPreview.tsx).

## API seam and target shape

Keep SoundAuditioner as the resource owner. Each run captures its controller, context, optional bank/player and sources; current-run identity governs admission and cleanup. A sound-workbench-local adapter exposes stopped/loading/playing snapshot with current key and optional live mix, replacing parallel UI flags. Preserve the stops notification consumed by children. Chatter keeps its own timeline/arbitration; use the same audition lifetime only at its existing resource seam.

## Transitions and invariants

New audition stops the prior run; preparation admits only to its captured current owner; playing has its captured resources and stop path. Edit, selection, tab change, explicit stop and unmount invalidate once. Old output completion cannot stop or mutate the replacement run. Preserve existing promise resolution/error behavior, calibration, permission resume, clipping and playback timing; a demonstrated stale mutation is a named correction, not silently normalized.

All [preservation contracts](../contracts.md) apply. No new wire/schema, dependency,
compatibility wrapper, timing policy or gameplay decision is introduced.

## Deletion and cutover

Delete generation/loadingAudio/playing/mix coordination where the run snapshot replaces it, and cleanup that reaches through mutable fields belonging to another run. Do not merge voice generation jobs, music policy or Chatter with audition.

Migrate consumers in this slice and remove the superseded representation at once.
No compatibility bridge is allowed. If the cutover cannot fit this slice, revise
the slice boundary so its consumers migrate together before implementation.

## Verification and runnable review

Focused existing gates: [audition.test.ts](../../../web/tests/audition.test.ts), [soundWorkbenchEditor.test.tsx](../../../web/tests/soundWorkbenchEditor.test.tsx), [voicesTab.test.tsx](../../../web/tests/voicesTab.test.tsx), [musicTab.test.tsx](../../../web/tests/musicTab.test.tsx).

Through injected audio/file completion, exercise stop and replacement at resume/module-load/bank/decode boundaries. Assert which current audio can start and which resources close, preserving live mix controls and child stop notification. Keep generation mocked and tests silent.

Audition a clip/live movement mix, change selection or tab while loading, stop, and restart chatter preview through the existing tool.

Follow [verification](../verification.md). If this slice produces a shot, explicitly
run `compare-screenshots` against the frozen before/approved target, then run an
unprimed `screenshot-critique` as the **last visual acceptance check**, and show the
useful evidence with `preview-shots`. The judged crop/variable is: Playback/loading controls and chatter timeline if captured; audio behavior uses command/resource evidence.
Everything outside that changed interaction/lifetime stays visually unchanged.
Human feedback uses the README's non-blocking review window.

## Decision budget and exit

Delegated: Private run record and adapter names. Current-run identity, stop scope and distinct chatter scheduling are fixed.

Feedback that would change scope: Crossfading auditions, parallel playback or replaying cancelled loads is outside this cleanup.

Exit only when the focused consumer contracts pass, affected type/lint checks are
green, obsolete ownership is removed, and the substantive-change review is complete.
Record evidence and decisions; update the README handoff before ending the pass.
