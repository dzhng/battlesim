# 16 · The yard swaps to the military set [held: L1]

Status: done 2026-11-13, commit b1429f076.

Map rows: Q2, Q5, L3. Depends on 11 and 15.

## Contract

The same `YardDocument` (slice 10), with the military kinds swapped in. **Only
data changes.** The yard reads as a lived-in military yard:
- a hangar line behind the apron;
- revetments around the motor pool;
- tents, sandbags and the flagpole by the infantry spot;
- nets only over supply stacks and containers, never over a spot (L3).

## Seam

Only `fixtures/armory/yard.json` changes.

## The human runs or sees

`/armory`, the same units as in slice 11, now in the military yard.

## Verification

Slice 10's yard tests, with no code change, including:
- no prop stands inside a spot's clear disc;
- every unit fits its spot.

**Visual gate.**
- **The one variable:** "a lived-in military yard".
- **Crop:** each spot's default camera, with the unit and the panels masked.
- `compare-screenshots`: the placeholder yard (slice 11's shots) against the
  military yard, then the military yard against the WARNO and Broken Arrow
  references.
- `screenshot-critique` last.
- Then the non-blocking `preview-shots` checkpoint.

## Delegated to the implementer

Placement.

## Must stay green

Slices 10, 11 and 12.

## Feedback that would change it

The layout is data.
