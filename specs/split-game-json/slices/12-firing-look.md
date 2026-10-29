# 12 The firing look follows the round fired

**Contract:** a hull's flash, firing sound and kick come from the round it
fired, not from its mount's first weapon. After this:
- the BMP-3's Bastion fires with the ATGM look;
- tank HE fires with the HE look.

**Seam:**
- `effects/launches.rs` hull launches, and the launch consumers in effects,
  pose and sound.
- **02 result: no conforming candidate.** A recovered all four BMP-3/tank
  rounds, but moved launches from ticks 2/9 to 3/10 because flight precedes
  fire (`sim/src/battle.rs`). Its effects vector moved (30 vs 38 instances
  at 0.05 s); sound/pose vectors held. Mount eligibility still needs resolving.
- B's last-fired `kind` injection kept ticks 2/9; the existing vector suites
  held, but this was an **oracle, not a production producer**. `kind` exists
  only in `effects::publication::Mount`; `sim/src/publication.rs::pose`
  publishes no round kind, and identified hulls expose no ammo. Hull counters
  and soldier segment matching also remain distinct. Prefer B's semantics
  only after approving its publication source and a shared matching rule.
- Both traces held all per-tick digests, replayed exactly, and passed
  `exact_numbers`. A moved only replay `engine_build`; B moved no replay bytes.
  No production mechanism is selected. Ask the user before adding a published
  round-kind field or authorizing another candidate. Constraints remain: no
  new published field; one matching rule; unchanged flash tick and attachment.

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
