# 11 Named fixes: kick, impact size, RPG level

**Contract:** three staging bugs the audit found are fixed, each as its own
commit with its own named delta.

1. **Tank guns kick.** No unit fires `tank_ap` directly, so derived tank
   rounds never kicked (`kick_of`'s exact-id match). With looks, every
   weapon on the tank looks gets the tank's hull kick, and the soldier kick
   covers ATGM, RPG, Igla and FN-6.
2. **Tank-round ground impacts at their intended size**, 5x size and 5x
   lifetime (`impacts.*.round_scale`). Today they show at 3x and 1x.
3. **RPGs back to their own loudness.** They have played the grenade's rows
   since the 2026-10-16 rename (`b4410901c`), about 11 dB quieter.

Each fix edits the values of [08](08-looks.md)'s transient bug looks
(`tank_sabot_no_kick` and the like) to match their siblings. No weapon
changes which look it names, so no hash moves. The looks that are now
identical are folded by [13](13-fold-looks.md).

**Run / see / hear:**
- a cropped clip of a tank firing, before and after (hull rock);
- impact plates from the look sheet;
- `firefight.wav` before and after, with the RPG's level in the ledger.

**Verify:**
- Before each re-bless, list the expected deltas: which rows, which outputs.
- Re-bless only those: `UPDATE_SOUND_VECTORS`, `BLESS_PARITY`, native plates.
- Battle digests unchanged.
- For the visual fixes, run
  [`compare-screenshots`](../../../.agents/skills/compare-screenshots/SKILL.md)
  before against after, then
  [`screenshot-critique`](../../../.agents/skills/screenshot-critique/SKILL.md)
  unprimed as the last check.
- Human checkpoint: open the clips and plates with
  [`preview-shots`](../../../.agents/skills/preview-shots/SKILL.md). It is
  non-blocking: about 5 minutes, then decide on the evidence, record it here,
  close the windows and go on.

**Moves:** presentation outputs only. `rules_hash` doesn't move, because no
weapon changes which look it names.

**Depends on:** [10](10-battle-rules.md).

**Delegated:** clip framing.

**Would change this slice:** the user finding a tank's rock or an impact too
big. Retune the look's values. That's a data edit and moves no hash.
