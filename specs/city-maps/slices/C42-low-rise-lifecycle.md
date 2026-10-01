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

Fallen occupants retain the owner of an elevated fighting floor after death,
even when the whole squad has relinquished its hold. The owner's terminal
transition removes that support: new collapse deaths and previously fallen
occupants settle at the existing walkable surface at their XY position, including
raised terrain and bridge decks. Identity and fall facing survive; settling adds
no survival roll. This private `Fallen.support_building` fact and optional observed
corpse snapshots enter the digest only when present, preserving ordinary-death
parity. The public corpse observation layout is unchanged.

Each side owns the enemy corpse pose it last observed. Own fallen follow the
physical support change immediately; an enemy body remembered above an unseen
collapse remains at its last observed height until sight returns. Once a floor
corpse has a remembered pose, it keeps that snapshot mode after physical support
is cleared. The pose driver refreshes animated, resting and fading anchors
without restarting death or bringing a faded body back; resting changes invalidate
its static publication version.

Native proof covers new elevated deaths on raised terrain, existing deaths in
surviving and wholly dead squads, hidden collapse and subsequent sight, and no
second survival roll. Deliberately omitting corpse settling fails at six metres
versus ground; publishing the live enemy pose fails the hidden-memory assertion.
The nearby 26 building tests retain the frozen parity oracles, and 33 pose driver
and feed tests pass, including anchor changes during death, rest and fading.
These are systems contracts; the pending prototype captures cannot certify
source-window fit or accepted building art.

[Structure systems evidence](../assets/sim-structures/README.md) preserves the
native support/knowledge proof, actual compound transition, complete raw-record
location and unprimed pixel critique. The prototype loses height visibly, but
its slab/rubble presentation, floor attachment, label overlaps and fog competition
remain qualified visual findings; this is not accepted source or art coverage.
