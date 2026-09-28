# C52: procedural generator

**Depends on:** C04 (last; first to cut). **Kind:** slice.

## Question
Can a procedural layout reuse the entire pipeline (Q1)?

## Contract it unlocks
`generate_city(LayoutConfig, seed) → CityPlan`, with SOURCES.json recording `project-owned` plus the seed. The same compiler, bake and validators as NYC.

## API seam
`crates/city-import`.

## What the human can run or see
A generated map report and an overlay.

## Verification
- Byte stability; connected streets; valid footprints; passes C05's tools within budget.

## Delegated to the implementer
One modest block/lot generator. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
No downstream source branches; no new styles.
