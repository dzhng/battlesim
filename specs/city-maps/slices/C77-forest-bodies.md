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


## Outcome — physical systems (2026-10-01)

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

Explicit systems inputs activate 5 log/3 boulder candidates per hectare, a
**named C77 digest change**. The default village stays disabled pending real
drawing. [Paired evidence](../assets/sim-forest-bodies/README.md) uses identical
final SA5 and C40–43 source with densities 0/0 versus 5/3: four logs and six
boulders are admitted, 40 candidates rejected, all 687 trunks retained. Startup
rises from 162.4 to 306.6 million instructions and peak RSS from 8.98 to 10.05 MiB.
The six-trial quick report is 1150→1109 G instructions; both arms capture all
three flank trials and no ambush trials. Flank seed 3 captures about 11 seconds
sooner; aggregate blue loss is 1662.5→1625 and rejoined soldiers 5→7. The five-minute
endurance is 1451.6→1449.9 G instructions and 259→260 MiB RSS, with a named digest
change and two additional soldier casualties. These are changed physical battles,
not a pure performance parity claim or balance tuning.

Both timing arms ran under substantial host load. The zero-floor control already
exceeds 33 ms; its maximum process CPU tick is 52.33 ms, versus 71.60 ms with cover.
The report therefore makes no 33 ms timing acceptance claim. Native instruction,
resource, reachability and rule gates are measured; active-variant timing and C78
models/cover GIFs/visual acceptance remain explicit follow-up gates. Manual
shape/diff/docs review and choices audit passed. The default CLI second review is
unavailable because its configured model is unsupported; independent guard review
and integrated whole-repo gates remain with the orchestrator.

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
pass. The overlapping-forest parity fixture explicitly supplies broader trunk spacing so accepted cover exercises both new kinds under all catalog mover patterns; dense overlaps may correctly admit no cover. A prop-only catalog test also passes without requiring named unit kinds. Paired startup instructions are recorded in the linked evidence; active-variant quiescent timing remains open.


### Default activation waits for accepted drawing

The canonical playable village retains zero floor densities. The 5 log/3
boulder candidates per hectare are explicit systems test/measurement inputs,
not active production placement. Pending `systems_only` bindings resolve to no
model; ordinary generated props are not building-part massing. Enabling them by
default would create invisible blockers. A strict release regression now refuses
active village floor density without accepted drawing, first red on the active
pending log and then green with default activation deferred. C78 must install
real accepted models/drawing before enabling the default. Prototype massing in
explicit systems lab evidence certifies only native geometry/state, never art.
