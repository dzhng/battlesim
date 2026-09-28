# C43: tall gutted

**Depends on:** C42. **Kind:** slice.

## Question
Does a building of >6 floors become a terminal gutted shell (Q4)?

## Contract it unlocks
At 0 hp:
- it keeps its height, blocking and occlusion;
- cover tier drops to medium;
- **it can't be garrisoned**;
- it can't be destroyed further;
- occupants take the collapse path once.

## API seam
The same lifecycle owner as C42.

## What the human can run or see
A GIF of a tall building gutting with its garrison escaping.

## Verification
- Native tests: never entered once gutted; no repeated destruction or survival rolls.
- Named digest change.

## Delegated to the implementer
None beyond implementation. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Low-rise behaviour; knowledge isolation.
