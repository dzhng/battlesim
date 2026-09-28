# C44: street bodies

**Depends on:** G0. **Kind:** slice.

## Question
Does each street prop behave by its war-film row (Q9)?

## Contract it unlocks
Catalog rows in `fixtures/props/city/*.json` (a prop kind is a catalog rank, `catalog.rs:213`). Kinds: lamp, bench, bollard, bins, hydrant, utility box, scooter, planter, parked car, car wreck, Jersey barrier, bus shelter, scaffold. A street tree is the existing tree body. Run tweak-mechanics **per kind**, and record a one-line war-film note per row. Rows only, no named cases.

## API seam
`fixtures/props/city/`, the catalog.

## What the human can run or see
One movement or fire scenario GIF per kind, for example: cover behind a car; a tank shoves a car; a bollard stops a jeep, not a tank; a glass shelter blocks movement but not rounds or sight.

## Verification
- Native test per row behaviour.
- Village digests unchanged (no village rows change).

## Delegated to the implementer
Per-kind values after the audit (in `choices.md`). Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Village rows.
