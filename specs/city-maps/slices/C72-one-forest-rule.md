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


## Stopping checkpoint

The [structural candidate](../assets/forest-shapes/README.md) is preserved as an
**unactivated** source/evidence snapshot on Root baseline `27e03c4`. Production
retains its current schema. Owned rectangle identity is independently proven:
57 native producer records are byte-identical, and the source-range placement
regression passes after a genuine duplicate-tree failure.

The seven-query GPU check remains red at a capsule endpoint (`-1` versus
`-1.0000001192092896`). Exact acceptance was not relaxed. Source/visual review,
paired pixels and remaining falsification gates are open. C65's sampled curves
and this slice's named uniform-density pass were not activated. Resume those
requirements after isolating the preserved GPU failure; this checkpoint does
not mark C72 complete.
