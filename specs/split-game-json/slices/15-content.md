# 15 Content pass

**Contract:** every look sounds and looks like its weapon. The firing sound
always fits the weapon, and burst recordings match each weapon's rate of fire.

Each item below is its own sub-slice: one variable, one verdict, one commit.
None moves a hash. The looks already exist from [13](13-fold-looks.md), so
only their values change.

**Sound** (sources from [03](03-sound-sources.md)):
1. firing recordings for `artillery`, `bomb` (release), the rocket salvo for
   `rocket_pod`, and the strike missile's look. Today they fall back to the
   rifle shot or the tank cannon;
2. mortar, if a mortar weapon uses the artillery look and the user can hear
   the difference;
3. burst recordings at cadence: the marksman rifle, the heavy sniper and
   `aa_gun`;
4. **the cadence check:** at fielding, a look whose burst recording doesn't
   match its weapon's rate of fire is refused (`sound/frame.rs:846-866`).
   Write it first; it fails on today's data, and passes once item 3 lands;
5. round the folded levels [13](13-fold-looks.md) wrote as raw medians
   (`3.0983866769659336`, far `825.0`) to the precision the file authors.
   Judge it by ear on `firefight.wav`, not by the ledger.

Record each library addition's provenance in `sounds.json` (source, author,
license, processing), as the catalog already does.

**Visuals** (judged on the look sheet):

6. `ifv_cannon`: distinct from the tank's HE (flash, tracer, impact);
7. `rocket_pod`: a rocket's motor, smoke trail and salvo impact, not a tank
   cannon's;
8. `rpg`: distinct from the grenade.

**Run / see / hear:** the look sheet (flash, tracer, impact and blast per
look), `firefight.wav`, workbench auditions ([14](14-sound-workbench.md)),
and the menu reel.

**Verify, per sub-slice:**
- Visual sub-slices: state the variable and the crop (that look's row on the
  sheet only). Other looks' wrongness is out of scope. Run
  [`compare-screenshots`](../../../.agents/skills/compare-screenshots/SKILL.md)
  against 13's sheet, then
  [`screenshot-critique`](../../../.agents/skills/screenshot-critique/SKILL.md)
  unprimed as the last check.
- Sound sub-slices: before/after WAVs and the source receipt.
- Re-bless only that look's vector outputs.
- Battle digests unchanged. The approved menu-reel shots keep their timing.
- Human checkpoint per sub-slice with
  [`preview-shots`](../../../.agents/skills/preview-shots/SKILL.md),
  non-blocking.

**Depends on:** [03](03-sound-sources.md), [13](13-fold-looks.md) and
[14](14-sound-workbench.md).

**Delegated:**
- crops and processing of sources;
- visual tuning within each look;
- a recipe from existing clips only where 03 found no source, recorded as a
  fallback.

**Would change this slice:** the user rejecting a recording. Pick the next
candidate from 03's ledger.
