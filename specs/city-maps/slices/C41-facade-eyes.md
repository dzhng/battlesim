# C41: facade eyes

**Depends on:** C40. **Kind:** slice.

## Question
Which seats supply a garrison's ≤4 eyes on any footprint shape?

## Contract it unlocks
`garrison::facade_eyes(SeatPlan, occupancy) → ≤4 eyes`. Exposed edges fall into **4 directional groups** relative to the building frame. Each group's eye is a **real occupied seat on its highest held band**, with stable tie-breaking and never averaged into a wall (S-eyes). This replaces the averaging at `garrison.rs:479-506`.

## API seam
`sim::garrison`, consumed by sensing, sweep and publication.

## What the human can run or see
A floor-3-over-shop scenario on a concave (L-shaped) building.

## Verification
- Vacant upper bands give no eye; ≤4 eyes; both sides' fog and identification agree.
- FOG_COST unchanged.
- Named digest change.

## Delegated to the implementer
Tie-break implementation. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Muzzles at seats.
