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
