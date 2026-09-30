# C46: street placement

**Depends on:** C44, C53. **Kind:** slice.

## Question
Does street placement keep playable space?

## Contract it unlocks
`place_street_props(MapPlan, catalog, seed) → props`, inside `crates/mapgen`: curb setbacks; no overlap with buildings, entrances or the reserved plain's open corridor; infantry and vehicle routes preserved; bounded rejection attempts. Street trees use C74's one tree generator and a named random stream.

## API seam
`crates/mapgen`.

## What the human can run or see
A block traversal GIF and a top-down density PNG.

## Verification
- Deterministic placement.
- Route-preservation test.
- City digests change (named).
- Frame-cost row.

## Delegated to the implementer
Density and spacing data. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Every placement resolves to a body and an appearance.

## Feedback that would change this slice
Furniture that blocks required approaches changes placement constraints before visual density is accepted.
