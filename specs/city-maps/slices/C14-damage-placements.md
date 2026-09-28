# C14: damage placements

**Depends on:** C13. **Kind:** slice.

## Question
Do the graphs' new damage inputs bake ruin and burnt tiers that follow the building's bays and floors?

## Contract it unlocks
Patched damage inputs (floors lost, facade breach, burnt; Q-G) produce **ruin** placements for ≤6 floors and **burnt/gutted** placements for >6 floors, fitted to C42's and C43's terminal geometry (ruin height = 25% of the building's, clamped 2–6 m).

## API seam
`packages/scene-assets/blender/city/` patch scripts; C13's placement schema gains `state`.

## What the human can run or see
Intact, ruin and gutted sheets per height class.

## Verification
- Byte identity.
- Ruin bounds within the sim's ruin box.

## Delegated to the implementer
Breach pattern; rubble distribution. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Sim owns damage timing and dimensions.
