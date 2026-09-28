# C05: measuring tools

**Depends on:** C04, C09. **Kind:** slice.

## Question
Does every later perf gate have a permanent, pinned tool with a 'before' row?

## Contract it unlocks
- `crates/sim/examples/city_report.rs`: build rows (world, nav, occlusion: ms and instructions), then an endurance-style battle on the city map with `endurance_report.rs`'s columns plus the publication split.
- A `CITY_CONTACT` benchmark preset (`web/src/battle/benchmark/scenario.ts`): pinned, a scripted street fight, and a strategic/pan/zoom/ground tour.
- `/lab/city-scale` and a `city-scale` scene.

## API seam
`crates/sim/examples/`, `web/src/battle/benchmark/`, `apps/battle-lab`.

## What the human can run or see
`/benchmark?preset=city-contact`, `/lab/city-scale`, the `city_report` table.

## Verification
- Baseline rows in `frame-cost.md` and a new `sim-cost.md`. They're expected to miss budget; this is the "before".

## Delegated to the implementer
Tour waypoints; the fight script. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
The village benchmark preset and digests.
