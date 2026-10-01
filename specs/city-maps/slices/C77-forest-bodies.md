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


## Outcome — physical core (2026-10-01; performance gates pending)

The log/boulder catalog rows, forest-rule type/density/dimension seam and bounded
seeded placement are implemented. Floor bodies are appended after **all** trunks,
so trunk IDs, source ranges, positions and foliage exports remain bit-identical
when floor density changes. Logs are medium destructible cover a tank can shove;
boulders are immovable heavy cover. Neither creates foliage or clears a lane.
The existing dynamic tree-fall rule is retained. [Choices](../choices.md#c77-forest-bodies--physical-core-2026-10-01)
records values and candidate-density semantics.

Focused native tests pass: movement/round/cover properties; trunk and foliage
parity with floor enabled/disabled; deterministic placement; load-time refusal
of invalid rules; and flood traversal of every open 2 m navigation cell for
infantry and jeep footprints on three forest geometries. The load-time regression
was falsified by removing validation, confirmed red for accepting an unknown
prop type, then restored green. The body shove matrix includes both floor kinds.
The appearance gate passes with explicit systems-only status; this does **not**
certify rendered playable cover.

The village explicitly activates the shared densities, a **named C77 digest
change**. Paired quick battle and endurance instruction evidence are pending the
SA5 production winner. The preserved original quick baseline took 287.4 s and
5026 G instructions; duplicating its slow sight path would obscure floor cost.
The next pass measures the same SA5 build with and without floor bodies before
calling the systems slice green. C78 models, cover GIFs and visual acceptance
remain open. Manual shape/diff/docs review and choices audit passed; the default
CLI second review is unavailable because its configured model is unsupported.
Integrated independent review and whole-repo gates remain with the orchestrator.


### Route-preserving admission follow-up

The spacing samples are regression evidence, not a universal connectivity proof.
Each candidate now enters one temporary shared navigation grid. Within its full
body-stamp/clearance influence window, every surviving cell of an old connected
component must remain connected and every previously open boundary cell must
remain open. The graph uses navigation's actual cell fit and infantry edge
crossings, with all actual catalog movers derived from current rules/catalog. A
rejected candidate is rolled back before the next candidate. This preserves
existing routes and cannot create an isolated pocket, even in a narrow corridor.

Generation builds the temporary grid once only when floor density is positive;
no full-grid flood or rebuild runs per candidate. A window exceeding 4096 cells
is rejected. Narrow-corridor rejection was first red with no guard, then green;
changed jeep hull width was deliberately falsified with a forced 1 m mover,
confirmed red, and restored. Accepted isolated cover and rollback tests also
pass. The overlapping-forest parity fixture explicitly supplies broader trunk spacing so accepted cover exercises both new kinds under all catalog mover patterns; dense overlaps may correctly admit no cover. A prop-only catalog test also passes without requiring named unit kinds. Startup instruction evidence remains part of the pending paired gate.
