# 14 Sound workbench keyed by look

**Contract:** the sound workbench auditions and saves a look's sound
(`weapon_looks.<look>.sound` in `presentation.json`). Library edits still go
to `sounds.json`, and mix edits to `audio.json`.

**Seam:** `apps/sound-workbench` and its server's publication.
- **Goes:** its `extends` walk (`server.ts:73`) and the delete-to-fallback
  button. There is no fallback any more.
- **Keeps:** the accepted-generation and outside-edit protections it already
  has.

**Run / see:** open `/sound-workbench`, pick a look, audition its shot,
blast, motor and impacts, change a level, save, reopen.

**Verify:**
- Rewrite `soundWorkbenchServer.test.ts` and `soundWorkbenchEditor.test.tsx`
  to this behaviour.
- New: a save changes only the intended document, and a no-op save leaves
  the bytes identical.
- Run
  [`screenshot-critique`](../../../.agents/skills/screenshot-critique/SKILL.md)
  unprimed on the workbench as the last check.

**Moves:** nothing. A look-value edit moves no hash.

**Depends on:** [13](13-fold-looks.md), so the workbench lists the final
looks.

**Delegated:** look-selector layout.

**Would change this slice:** the user wanting to change which look a weapon
names from the workbench. That's a `rules.json` edit, so ask first.


## Done — 2026-10-29

The Weapon looks tab selects a final look, auditions shot near/far, blast near/far,
motor and material contacts, and saves only its authored sound block in
`presentation.json`. Library drafts still save to `sounds.json`; mix views remain
read only and their shared choices remain in `audio.json`. There is no look
inheritance walk or delete-to-fallback control. Rules and look assignments are
untouched. Combined library/look saves use the existing reviewed candidate and
publication journal; outside edits and interrupted writes retain their protection.

Validation: tests were written first and observed red. The focused workbench and
music suites pass (43 tests). The complete web run plus affected-suite retries
passes every discovered suite; missing LFS art, launcher source and effect sheets
were fetched by their required paths. WASM build, typecheck, lint and the staged
size ratchet pass; lint retains existing warnings outside this slice. No Rust crate
changed. The exact proof matches `after-13-merged/` for all eight artifacts,
including sound, effects, poses, hull publication and battle digests. No fixture
values, rules hashes or replay configuration identities move. Unchanged renderer
and fixture inputs reuse slice 13's native-plate verification.

Shared evidence: `/Users/server/dev/battlesim/throwaway/split-game-json/evidence-14/`.
`picked-look.png` and `saved-reopened.png` show the real route. The browser proof
plays every sound part, saves a level, reopens it, checks the intended document
and key, checks no-op bytes and restores the original fixture bytes. The historical
comparison, enlarged crops, visual review and final unprimed critique are kept
there. Preview shows the selected and reopened captures together for the
non-blocking visual checkpoint; acceptance follows their readable controls and
isolated-save proof. The critique report records the independent verdict.

Two pre-existing real-route defects were fixed in a separate commit: the SSR
validator instance needed initialization, and preview refused the library's
external provenance receipts. Receipts now remain immutable like source metadata.
Voice/music server tests were moved into their own file before expanding sound
publication coverage, keeping both files within the source-size budget.

The first unprimed critique identified clipped recipe descriptions and a wandering
impact-selector/button boundary. Recipe selectors now show their unique id with
the complete human description wrapping beneath, and material audition buttons
share a column. Captures and the final critique were refreshed after those fixes;
the initial evidence remains in `evidence-14/attempt-1/`.

Next: [15 Content pass](15-content.md). No sound values were retuned here.

**Review:** a read-only review of the save destinations, protections, no-op saves, leftover fallback code and tests found nothing.
