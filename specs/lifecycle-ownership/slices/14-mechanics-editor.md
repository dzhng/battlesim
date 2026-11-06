# 14 — Mechanics draft and publication session

**Status:** pending. **Audit families:** 75. **Dependencies:** none.

## Contract and question

Can review/save/reload transitions leave JSX handlers while retaining unfinished input and source conflicts?

Current owners/consumers: [MechanicsEditor.tsx](../../../apps/mechanics-editor/src/MechanicsEditor.tsx), [draft.ts](../../../apps/mechanics-editor/src/draft.ts).

## API seam and target shape

Add a mechanics-local pure reducer owning snapshot, logical draft, unfinished text, review certificate, operation/outcome and discard confirmation. Keep field validation/draft utilities authoritative; search/expanded selection and DOM last-edit focus remain view state. A narrow hook/controller executes existing HTTP requests and dispatches their results; no effects or IO in the reducer.

## Transitions and invariants

Edit invalidates review and saved outcome according to current rules; preview binds exact draft JSON; save requires current valid reviewed draft and resets only after acceptance; failure retains edits, and conflict persists until explicit reload. Dirty reload first arms discard. Preserve subtree text cleanup and fallback to the last connected edit for native rejection.

All [preservation contracts](../contracts.md) apply. No new wire/schema, dependency,
compatibility wrapper, timing policy or gameplay decision is introduced.

## Deletion and cutover

Delete repeated save/reload reset bundles and broad busy/review/conflict mutations from handlers. Do not build a generic editor reducer or replace the server/fixture publication owner.

Migrate consumers in this slice and remove the superseded representation at once.
No compatibility bridge is allowed. If the cutover cannot fit this slice, revise
the slice boundary so its consumers migrate together before implementation.

## Verification and runnable review

Focused existing gates: [mechanicsEditor.test.tsx](../../../web/tests/mechanicsEditor.test.tsx), [mechanicsEditorDraft.test.ts](../../../web/tests/mechanicsEditorDraft.test.ts), [mechanicsEditorFields.test.ts](../../../web/tests/mechanicsEditorFields.test.ts).

Use mounted editor tests for invalid text through filters, current review before save, conflict retention, subtree restore and last-edit focus. Run mechanicsServer/mechanicsPublication tests only as required by the integrated boundary; no new field/config string assertions.

Edit a value and invalid text, filter away/back, preview/save, simulate a changed source receipt, then explicitly discard/reload.

Follow [verification](../verification.md). If this slice produces a shot, explicitly
run `compare-screenshots` against the frozen before/approved target, then run an
unprimed `screenshot-critique` as the **last visual acceptance check**, and show the
useful evidence with `preview-shots`. The judged crop/variable is: Edited field, preview and error/discard panels; existing labels/styles/layout frozen.
Everything outside that changed interaction/lifetime stays visually unchanged.
Human feedback uses the README's non-blocking review window.

## Decision budget and exit

Delegated: Private reducer event names and small cohesive view subcomponents. Domain drafts, request payloads, review identity and conflict policy are fixed.

Feedback that would change scope: Autosave, different discard confirmation or a new validation policy would change the product contract.

Exit only when the focused consumer contracts pass, affected type/lint checks are
green, obsolete ownership is removed, and the substantive-change review is complete.
Record evidence and decisions; update the README handoff before ending the pass.
