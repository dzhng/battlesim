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


## Outcome

**Physical half done; the look rows wait.** `contract::map::SurfaceKind` is `road | country_road | dirt_track | sidewalk`, and the rules' `surfaces` table (`contract::scenario::SurfaceRule`) gives each a `speed_factor`: road and country road 1, dirt track 0.75, sidewalk 0. The rules refuse a missing row or a factor outside 0..=1. The world's surface query carries the factor (`Surface::road_factor`), and navigation and both movers read that one value: a mover travels at its own road speed times the factor, never slower than on open ground.

- Village outcome digests are identical (`village_report --quick`, all six trials); the rules gained a section, so `config_digest` changes. Total instructions 5,251 G against 5,246 G.
- `t3-jeeps-country-road-vs-dirt-track`: 18.0 m/s on the road, 13.5 m/s on the track.
- Still open for the visual pass: the biome's `roads.<kind>` look rows (C66), and a distinct exported tag per road kind. Every carriageway exports as a road today.
