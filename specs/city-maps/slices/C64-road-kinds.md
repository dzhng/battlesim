# C64: road kinds

**Depends on:** C63. **Kind:** slice.

## Question
Does each rural road kind carry its own data-driven speed, with the village unchanged (Q-G4)?

## Contract it unlocks
- C03's road surface gains `kind: country_road | dirt_track`.
- **One surface-kind table** in the rules, `surfaces.<kind>.speed_factor`, covers every surface kind C03 defines plus the two rural road kinds. It multiplies the per-unit-type road speed (`units.rs:263-266`). Country road = 1.0, so the village keeps today's speeds.
- Navigation's road boolean (`navigation.rs:72, 107, 198`) reads the surface row.
- The biome gains `roads.<kind>` look rows, unused until C66.

## API seam
`contract::map`, `sim::navigation`, `sim::movement`, the rules.

## What the human can run or see
A movement-lab GIF of a jeep on a dirt track against a country road.

## Verification
- Run tweak-mechanics first (the dirt-track factor).
- Native test per kind.
- **Village outcome digests identical**; the `config_digest` header change is named.
- `NavGrid::build` instructions equal or lower.

## Delegated to the implementer
The dirt-track factor (recorded in `choices.md`). Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Village speeds and outcomes.

## Feedback that would change this slice
An implausible surface speed difference reopens the fixture-owned kind table through the rules workflow.
