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

## Feedback that would change this slice
A tall terminal shell that appears collapsed reopens state/body/appearance alignment; the standing-gutting threshold is retained.


## Outcome — physical implementation checkpoint (2026-10-01)

The fixture destruction policy chooses a terminal gutted state above six source
floors. Every aggregate part retains its full source height; the body keeps
movement blocking, sight occlusion and round stopping with medium cover, but
has no HP or garrison. The existing single aggregate collapse path releases
occupants once. A native battle checks the full-height terminal replacement,
occupant exit and rejection of another transition or garrison order.

The gutted appearance explicitly awaits art. Legacy boxes without source floor
facts cannot certify the tall threshold. Specialist art/GIF gates and integrated
checks remain open.

A terminal shell's retained exterior height does not retain fighting-floor
support. Fallen occupants use the same support-loss and side-specific corpse
memory contract as [C42](C42-low-rise-lifecycle.md); the visual body follows the
published pose without replaying its death. This does not certify a charred
exterior, window geometry or accepted tall-building source.
