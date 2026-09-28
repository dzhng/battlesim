# C46: street placement

**Depends on:** C44, C04. **Kind:** slice.

## Question
Does street placement keep playable space?

## Contract it unlocks
`place_street_props(CityPlan, catalog, seed) → props`, inside the importer: curb setbacks; no overlap with buildings or entrances; infantry and vehicle routes preserved; bounded rejection attempts. Street trees come from the census where available.

## API seam
`crates/city-import`.

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
