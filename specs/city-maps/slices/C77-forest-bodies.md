# C77: forest bodies

**Depends on:** C72. **Kind:** slice.

## Question
Is the cover you see on a forest floor real (Q-G12)?

## Contract it unlocks
- Catalog body rows `fixtures/props/forest/{log, boulder}.json`, set by a war-film audit per kind.
- **Generated sparsely by the one forest rule**: `logs_per_ha` and `boulders_per_ha`, seeded per forest, placed after the trunks in trunk-free gaps so no trunk moves.
- A fallen trunk exists only as a log body.
- **Named village digest change.**

## API seam
`fixtures/props/forest/`, `sim::world::forest`, `forests.rule`.

## What the human can run or see
GIFs of a squad taking cover behind a log, and of a boulder stopping a jeep.

## Verification
- Run tweak-mechanics per kind; a test per ruled-out moment.
- Trunk positions identical with and without bodies.
- Every forest cell stays reachable.
- `--quick` report; endurance instructions.

## Delegated to the implementer
Densities, sizes and the audit's values (recorded in `choices.md`). Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Nav through forests.

## Feedback that would change this slice
A forest body that blocks or shelters unexpectedly reopens its physical property/placement rule before model polish.
