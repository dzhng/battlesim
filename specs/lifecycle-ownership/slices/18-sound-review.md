# 18 — Sound draft review and save ownership

**Status:** complete. **Audit families:** 78. **Dependencies:** none.

## Contract and question

Can sound catalog/look editing own its review certificate independently of audition state?

Current owners/consumers: [SoundWorkbench.tsx](../../../apps/sound-workbench/src/SoundWorkbench.tsx), [protocol.ts](../../../apps/sound-workbench/src/protocol.ts), [server.ts](../../../apps/sound-workbench/server.ts).

## API seam and target shape

Add a sound-local pure draft reducer carrying accepted snapshot, catalog/look draft, reviewed candidate and active preview/save/reload operation/outcome. A narrow controller runs the existing API. Selection/tab/search and audition stay separate. Server-held review/revision checks and VoiceWorkbenchStore remain current owners; do not make an editor abstraction shared with mechanics/maps.

## Transitions and invariants

Accept snapshot clones the same editable documents; edit invalidates review/message and requests the existing audition stop; preview validates and captures its review; save uses exact reviewed candidate/revision; failed/stale save retains the draft; successful save/reload adopts a new snapshot and resets only the existing fields. Preserve validation and explicit discard behavior.

All [preservation contracts](../contracts.md) apply. No new wire/schema, dependency,
compatibility wrapper, timing policy or gameplay decision is introduced.

## Deletion and cutover

Delete repeated accepted-snapshot reset and review invalidation bundles from handlers; keep domain-specific draft differences and the existing server candidate.

Migrate consumers in this slice and remove the superseded representation at once.
No compatibility bridge is allowed. If the cutover cannot fit this slice, revise
the slice boundary so its consumers migrate together before implementation.

## Verification and runnable review

Focused existing gates: [soundWorkbenchEditor.test.tsx](../../../web/tests/soundWorkbenchEditor.test.tsx), [soundWorkbenchServer.test.ts](../../../web/tests/soundWorkbenchServer.test.ts).

Preserve stale review/source receipt refusal, retained draft, edit invalidation, no-op source bytes and saved look reopening. Use the mounted editor through API edges. Do not invoke paid generation, change fixture schemas or rewrite publication recovery.

Edit catalog/look sound settings, preview, save, reopen, and retain the draft after a deliberately stale review.

Follow [verification](../verification.md). If this slice produces a shot, explicitly
run `compare-screenshots` against the frozen before/approved target, then run an
unprimed `screenshot-critique` as the **last visual acceptance check**, and show the
useful evidence with `preview-shots`. The judged crop/variable is: Draft controls, preview and save error/review panel; sound assignments and UI look frozen.
Everything outside that changed interaction/lifetime stays visually unchanged.
Human feedback uses the README's non-blocking review window.

## Decision budget and exit

Delegated: Private reducer/events and local hook placement. Review identity, explicit save/discard and server formats are fixed.

Feedback that would change scope: Autosave, shared generic draft semantics or different conflict resolution requires a product change.

Exit only when the focused consumer contracts pass, affected type/lint checks are
green, obsolete ownership is removed, and the substantive-change review is complete.
Record evidence and decisions; update the README handoff before ending the pass.

## Implementation receipt

The sound workbench now has a sound-local pure reducer for the accepted snapshot,
catalog/look drafts, review certificate, operation identity and publication
outcome. The component controller runs the existing API and keeps selection,
tabs/search and audition state separate. Catalog and look edits have separate
reducer events, so one edit cannot overwrite the other document from a stale
render closure. Music job completion refreshes after its state update, and a
refresh cannot replace an active preview/save operation. No protocol, server
candidate, fixture schema or audition resource owner changed.

Focused evidence from the working tree:

- `bun run --cwd web test -- tests/soundWorkbenchEditor.test.tsx tests/soundWorkbenchServer.test.ts` — 37 tests passed (15 editor, 22 server).
- `bun run --cwd web typecheck` — passed.
- `oxlint` on changed TypeScript files — passed.
- `oxfmt --check` on changed TypeScript files and `git diff --check` — passed.
- Mutation check: temporarily clearing the draft in the failure transition made
  the mounted failed-preview test fail; restoring the transition made it pass.

Fresh Claude Opus high-effort review and re-check: [final review thread](<t3-thread://v2/9950c6b4-6db9-4544-9904-e8f629486d1f/thread:delegated-task:command%3Amcp%3Ad0a00d42-5e07-45cd-863b-3487d797e820%3Adelegate-task%3A09989e18-96c3-4a6f-a07a-b97a0dae0c44>). The first review found stale audition-message clearing, refresh races, late unmount cleanup, duplicated save handling, stale cross-document edit payloads, and a weak regression test. All findings were fixed; the final re-check verdict was clean with no blockers. No visual shot was produced, so the visual comparison gate was not applicable.

Production size is 143 added and 116 deleted lines in the existing workbench and
music tab, plus a 166-line sound-local reducer. Tests add 32 lines and delete 1.
No public format or server contract changed.
