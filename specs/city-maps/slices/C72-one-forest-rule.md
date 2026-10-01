# C72: one forest rule

**Depends on:** C03 shared shape primitives and GG. **Kind:** slice.

## Question
Does every forest play by one density and one tree size, so the art can't lie about sight (Q-G8b)?

## Contract it unlocks
- `Forest` keeps only its physical shape, using the contract's shared polygon/stroke representation; existing rects become equivalent polygons. Preserve existing trunk identities where geometry/rule are unchanged. This structural shape contract precedes C04. One rules row, `forests.rule`, holds today's `medium` values: 9 m spacing, jitter, concealment, attenuation, 6.5 m canopy radius, 12 m canopy, 10 m trunk and clearance. `ForestDensity` and `densities` are deleted.
- Existing medium rectangular forests retain the same trunks through the shape-only cutover; C52 new polygon/strip forests use the pinned deterministic generation contract. S0/G0 governs full-extent foliage storage; varying sparse woodland changes coverage rather than this rule.
- **Named changes on 5 fixtures:**
  - village: the east wood goes light → medium (about 146 → about 355 trunks);
  - endurance: light → medium (the sim perf yardstick is re-baselined that day);
  - `sensors` lab: dense → medium (its thin/deep forest tour becomes edge/deep);
  - `geometry` lab: dense → medium;
  - `benchWorld.ts:253-259`.

## API seam
`contract::{map, scenario}`, `sim::world::forest`.

## What the human can run or see
A GIF of a recon squad at the former light wood's edge, before and after.

## Verification
- Run tweak-mechanics first.
- Tests on the rule.
- `village_report -- --quick --compare main`.
- Endurance instructions recorded as the new baseline.
- Replays re-baselined, named.

## Delegated to the implementer
None: the values are today's medium; any change waits for C50. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Foliage and fog agreement (≥95%).

## Feedback that would change this slice
A forest that implies a different density rule by species reopens shared coverage/scale before art tuning.


## Outcome

**Done.** `Forest` is only a shape (`contract::map::Forest`; the shared polygon/stroke `GroundShape`), and `forests.rule` (`contract::scenario::ForestRule`) holds today's medium spacing, jitter, concealment, attenuation and crown plus the tree's size: canopy 12 m, trunk radius 0.35 m, height 10 m, clearance 2 m. `ForestDensity`, `densities` and the per-forest tree fields are gone, and a forest row carrying any of them is refused at load. The sim reads the rule from `ForestState`; a tree carries only a `canopy` flag.

- The shape cutover kept original trunk IDs (57 native producer records byte-identical at the candidate's baseline).
- The GPU forest query matches the sim's inside/outside answer at every probe and its distance to 0.1 mm; see `choices.md`.
- The named changes landed: the village east wood, endurance, the `sensors`, `geometry`, `consequences` and `movement` labs, and the workbench now all use the one rule. Frozen parity outputs that depended on light or dense woods were regenerated (`BLESS_PARITY=1`); everything else stayed byte-identical.
- Movement scenarios: `t1-spotted-open-vs-forest` (open ground spotted at 598 m, the forest at 207 m) and `t3-jeep-through-forest` (a jeep still threads the forest, knocking nothing).

**Measured shift (balance waits for C50).** `village_report --quick` against main: every digest moved, as named. The flank script captures 2 of 3 seeds in 600 s where it captured 3 (seed 1 took 540 s before and now runs out the clock), and blue's cost lost rises 728 → 790. The ambush script is unchanged at 0 of 3 with 120 lost.

Main's first ten minutes cost 23,579 G; this slice's cost 25,215 G (+6.9%, the denser wood).

**Endurance baseline (60 minutes, step instructions in billions per five minutes):** 10,853 · 14,362 · 9,536 · 3,807 · 1,615 · 1,562 · 1,552 · 1,556 · 1,550 · 1,557 · 1,551 · 1,557.
