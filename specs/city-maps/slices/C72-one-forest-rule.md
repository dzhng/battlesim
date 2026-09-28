# C72: one forest rule

**Depends on:** GG. **Kind:** slice.

## Question
Does every forest play by one density and one tree size, so the art can't lie about sight (Q-G8b)?

## Contract it unlocks
- `Forest` keeps only its shape. One rules row, `forests.rule`, holds today's `medium` values: 9 m spacing, jitter, concealment, attenuation, 6.5 m canopy radius, 12 m canopy, 10 m trunk and clearance. `ForestDensity` and `densities` are deleted.
- The forest seed hash is unchanged, so already-medium forests keep their trunks.
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
