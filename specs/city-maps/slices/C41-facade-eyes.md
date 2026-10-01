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

## Feedback that would change this slice
A floor-band eye that cannot see as a viewer expects reopens eye/seat geometry through the rules workflow.


## Outcome — physical implementation checkpoint (2026-10-01)

Each direction publishes one actual living occupied seat on its highest held
floor, with stable seat-order ties. Empty directions publish no eye. No facade
averages or unoccupied windows contribute sight. Native checks pin the real seat
origins and demonstrate third-floor identification above a lower obstacle.
Existing exterior-union and range contracts remain checked. The building parity
oracle was regenerated for the named seat/eye/digest change; its input is frozen.

Source facade fit and specialist visual acceptance remain open. Integrated
native/Wasm parity and whole-repo gates are pending.
