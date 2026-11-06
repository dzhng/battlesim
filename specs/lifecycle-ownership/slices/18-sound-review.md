# 18 — Sound draft review and save ownership

**Status:** pending. **Audit families:** 78. **Dependencies:** none.

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
