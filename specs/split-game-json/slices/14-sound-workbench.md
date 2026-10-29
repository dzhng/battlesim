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
