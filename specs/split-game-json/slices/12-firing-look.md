# 12 The firing look follows the round fired

**Contract:** a hull's flash, firing sound and kick come from the round it
fired, not from its mount's first weapon. After this:
- the BMP-3's Bastion fires with the ATGM look;
- tank HE fires with the HE look.

**Seam:**
- `effects/launches.rs` hull launches, and the launch consumers in effects,
  pose and sound.
- The mechanism is the one [02](02-hull-launch-spike.md) chose, written here
  when 02 finishes. Constraints: no new published field; soldiers and hulls
  share one matching rule; flash timing and muzzle attachment unchanged.

**Run / see / hear:** the look sheet's firing rows, and a clip of a BMP-3
firing its gun then its Bastion, and a tank firing AP then HE.

**Verify:**
- Write first: a presentation test in which a two-weapon mount fires its
  second row, and the launch carries that row's look. Cover:
  - a moving, turning hull;
  - a skipped publication;
  - a round whose first stretch is clipped by fog;
  - an immediate impact.
- List the expected deltas, which are hull firing effects of non-first
  weapons only. Re-bless those and nothing else.
- Battle digests unchanged.
- Run
  [`compare-screenshots`](../../../.agents/skills/compare-screenshots/SKILL.md)
  before against after, then
  [`screenshot-critique`](../../../.agents/skills/screenshot-critique/SKILL.md)
  unprimed as the last check.
- Human checkpoint with
  [`preview-shots`](../../../.agents/skills/preview-shots/SKILL.md),
  non-blocking as in the README.

**Moves:** presentation outputs for non-first hull weapons. If 02 chose to
fill a published value, the publication bytes move too; battle digests don't.

**Depends on:** [02](02-hull-launch-spike.md) and [10](10-battle-rules.md).

**Delegated:** the implementation inside the chosen mechanism.

**Would change this slice:** 02 finding no mechanism without a new field.
Ask the user before adding one.
