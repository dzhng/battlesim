# C42: low rise lifecycle

**Depends on:** C01. **Kind:** slice.

## Question
Does a building of ≤6 floors collapse exactly once as one structure, sized to its bulk (Q4)?

## Contract it unlocks
- hp = coefficient × footprint area × banded floors, replacing the flat 400 (`structures.json`).
- Ruin height = 25% of the building's height, clamped 2–6 m, from the `destroyed` row, not a code branch.
- Blast damage is deduplicated across parts.
- One survival and escape path for occupants.
- **Named village digest change.**

## API seam
`sim::structures`, the building body row, `destroy_prop`.

## What the human can run or see
A GIF of a multi-part ≤6-floor collapse.

## Verification
- Run tweak-mechanics first.
- Native tests: shared damage; atomic swap; ruin height formula; one survival roll per occupant.
- `--quick` report.

## Delegated to the implementer
Provisional coefficient as fixture data (final in C50). Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
One integrity owner; deterministic RNG order.

## Feedback that would change this slice
An implausible collapse moment reopens the retained lifecycle contract only as a named game-rule decision.


## Outcome — physical implementation checkpoint (2026-10-01)

Building integrity is fixture coefficient × union footprint area × first-three
floor bands. The immutable aggregate caches its union area, so overlap is counted
once and damage has constant-cost integrity lookup. Scaling belongs to physical
building bulk, independently of whether that building permits garrisoning.
Ordinary props cannot request aggregate scaling without building facts.

The destroyed row specifies 25% height clamped 2–6 metres. Existing shared damage,
atomic replacement and one occupant survival/escape path remain the lifecycle
owners. Native checks exercise bulk HP, low-rise heights and aggregate collapse.
Coefficient 1 is provisional for C50; village changes are named rule changes,
not parity-preserving performance work. Quick-report and integration gates are
pending. The collapse GIF and final source fit remain open.

Aggregate loading enforces immovable bodies through every declared replacement
state and its terminal gutted alternative. A catalog-valid building-to-crate
chain was first accepted incorrectly, then refused after the guard; ordinary
movable crates still load and move. This preserves the existing no-composite-motion
boundary rather than leaving immutable source seats attached to independently
shoved parts. All 25 aggregate-building regressions pass on the integrated source.
