# 19 — Audition operation and resource ownership

**Status:** complete. **Audit families:** 79. **Dependencies:** 18.

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

## Implementation receipt

Baseline: `f7c0c65b8` (sound review closeout). The workbench now projects one local
audition session snapshot (`stopped`, `loading`, or `playing`, with its current key,
live mix, and stop notification) while `SoundAuditioner` owns one complete resource
run. Replacement and stop admission use captured run identity; voice and chatter
previews continue through their existing line/voice seams. No wire, publication,
generation, or timing contract changed.

Named correction: **stale audition resume/module-load ownership**. A replacement
could let an older resume/module-load completion create a stale bank or player after
the newer run was current, leaking resources and allowing the wrong owner to be
disposed. The run record, abort admission before bank creation, and per-run cleanup
fix that ordering. The focused module-load test fails against the pre-slice code by
showing a bank used without a matching disposal; the candidate also covers replacement
during bank decode and proves the old context/bank closes before the deferred decode
resolves. A separate music-player test proves an orphan player is disposed and the
replacement rejects with `AbortError`.

Named preservation correction: MusicTab now consumes `AbortError` silently when a
music audition is cancelled or replaced while loading. Real playback errors still
reach the existing message path; the change avoids exposing an expected lifecycle
abort as an error after the resource owner has already stopped.

Focused evidence:

- `bun run --cwd web test -- tests/audition.test.ts tests/auditionSession.test.ts tests/auditionMusic.test.ts tests/soundWorkbenchEditor.test.tsx tests/voicesTab.test.tsx tests/chatter.test.ts tests/musicTab.test.tsx` — 37 passed.
- `bun run --cwd web typecheck` — passed after generated WASM artifacts were built.
- `bun run fmt:check` and `bun run lint` — passed; lint reports only existing unrelated warnings.
- Changed-file Oxlint, ESLint complexity, Oxfmt, and `git diff --check` — passed.
- Mutation cases cover late replacement completion, explicit stop during loading,
  stale module-load, bank/decode completion, orphan music-player disposal, and the
  cancellation-silence test fails if MusicTab reports `AbortError` again.

The focused voice suite emits jsdom's existing `window.confirm` not-implemented
console message while passing; it is from the unchanged voice tab. No visual surface
changed, so no screenshot evidence was produced. Claude Opus high-effort read-only
acceptance re-check was clean at
<t3-thread://v2/9950c6b4-6db9-4544-9904-e8f629486d1f/thread:delegated-task:command%3Amcp%3Afb8cabd0-71aa-483e-bdcb-ec55c9186c63%3Adelegate-task%3A50a5833d-7efe-4672-9ac5-135da423581f>.
