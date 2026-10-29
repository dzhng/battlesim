# 13 Fold to the final looks

**Contract:** weapons name the final set of looks. Near-duplicates are
folded, and the looks the content pass needs exist, so [15](15-content.md)
only edits look values and never moves a hash.

**The final assignment (about 18 looks):**
- **Fold** sound nudges under 2 dB into one level per look: tank AP ×4,
  ATGMs ×9, SAMs ×6, MANPADS ×3, rifle and carbine.
- **Split** where the difference is real:
  - `ifv_cannon` (the BMP-3's 100 mm main gun) from `tank_he`: 9 dB;
  - `aa_gun` from the HMG: 7 dB;
  - `bomb` from `artillery`: 20 dB;
  - `rpg` from the grenade: 11 dB.
- **Own looks for the content pass**, holding today's values until 15
  changes them: `rocket_pod` (today it looks and sounds like a tank cannon),
  and the strike missile's look (today it fires the rifle's shot).
- **Names** say what the look is (`tank_sabot`, `tank_he`, `ifv_cannon`,
  `atgm`, `manpads`, `sam`, `aa_gun`, `artillery`, `bomb`, `rpg`, `rifle`,
  …), never one of the weapons that share it.
- Every transient look from 08 is gone.

**Seam:** weapon rows' `appearance` in `rules.json`, and
`presentation.json` `weapon_looks`.

**Run / hear:**
- A level ledger: for every weapon, its old effective gain, its new gain,
  and the difference. Folds stay under 2 dB; splits are as listed.
- `firefight.wav` before and after.
- The look sheet.

**Verify:**
- The probe diff from 01 shows only the ledger's gain changes and the named
  splits.
- The fixture test: every look is used.
- Battle digests unchanged.
- Run
  [`compare-screenshots`](../../../.agents/skills/compare-screenshots/SKILL.md)
  on the sheet, then
  [`screenshot-critique`](../../../.agents/skills/screenshot-critique/SKILL.md)
  unprimed as the last check.
- Human checkpoint with
  [`preview-shots`](../../../.agents/skills/preview-shots/SKILL.md) and the
  WAVs, non-blocking.

**Moves:** `rules_hash` and replay config digests, once. Rewrite the stored
records with `throwaway/split-proof/rewrite-look-folds.mjs`, then re-bless
the sound and effect vectors as named changes.

**Depends on:** [11](11-named-fixes.md) and [12](12-firing-look.md).

**Delegated:** the level each folded group keeps, justified by the ledger (the
loudest member's level, or the median; say which and why).

**Would change this slice:** a fold the user can hear as wrong. Split it into
its own look.
