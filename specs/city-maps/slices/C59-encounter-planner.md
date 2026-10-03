# C59: map-aware encounter placement

**Depends on:** C58, C33, C53, C09, C40's seats/capacity and G0's access/range criteria. **Kind:** slice.

## Question
Can one encounter recipe place legal forces, objectives and defenders on any supported compiled map?

## Contract it unlocks
Extend the existing scenario builder with a deterministic entry point:

```text
plan_encounter(public_map_queries, resolved_rules, EncounterRecipe, encounter_seed)
  → EncounterDefinition | stable diagnostics
```

`EncounterRecipe` specifies the roster, mission/variant, settlement/approach preferences and G0's deployment/objective constraints. It contains intent, not village coordinates. `EncounterDefinition` holds legal team deployments, objectives/capture zones, defender inputs and initial garrison references to actual compiled buildings. Map-generation seed and encounter/battle randomness are separate pinned inputs.

The builder reads compiled urban/plain geometry and authoritative sim terrain/body/navigation/seat queries. It selects reachable spawn/objective regions, places each footprint clear of bodies/water/other units, admits whole squads within garrison capacity and derives defender/benchmark waypoints from those anchors. Stable ordering and bounded candidate attempts give reproducible results; impossible placement returns diagnostics rather than an invalid battle.

Save the exact prepared encounter with reviewed fixed-seed artifacts. Runtime calls the same planner after map resolution and records the result with the compiled map/rules/identity in replay inputs. The sim consumes its existing resolved scenario shape. There is no generator-specific battle, second rule engine or runtime-only placement algorithm.

## API seam
Existing sim scenario-builder owner plus contract-typed recipe/result data; CLI/wasm preparation orchestrates it. Mapgen core remains contract-only. Preparation/validation drivers may use sim queries; they do not reimplement navigation/body rules. Reuse the prepared geometry query owner within G0's memory/startup contract rather than retaining a second full world.

## What the human can run or see
A placement overlay across type/size/seed maps: deployments, capture zones, assigned garrisons and approach/defender waypoints, with stable rejection reasons. Start battles from accepted prepared encounters.

## Verification
- Native/wasm canonical recipe/result/diagnostic parity and stable building references.
- The C58 map, one fixed seed per type × size cell and selected impossible/edge cases satisfy footprint clearance, required routes, objective reachability and whole-squad garrison admission.
- C54 owns the multiple-seed integration matrix. The prepared encounter demonstrates useful 1,800 m approaches; unused open space alone cannot satisfy that criterion.
- Roster/mission variants are deterministic and bounded; unavailable legal regions/buildings yield failure.
- Saved/runtime preparation with matched inputs yields the same resolved scenario/replay identity; startup queries fit G0's budget.
- Compare placement overlays with physical route/seat evidence using compare-screenshots; run unprimed screenshot-critique last. Preview-shots is non-blocking.

## Delegated to the implementer
Recipe defaults and reversible deployment/objective choices within inherited physical/access/range constraints; record them in choices.md. New combat rules and source-specific placement algorithms are not delegated.

## Must stay green
One scenario builder, authoritative physical queries and exact prepared replay inputs.

## Feedback that would change this slice
Poor encounter play changes its recipe or bounded selection policy; it cannot invalidate physical constraints or shrink the map.

## Outcome

The planner is `sim::encounter::plan_encounter` ([module](../../../crates/sim/src/encounter/mod.rs)), and `/lab/generated` plays what it plans. The TypeScript stand-in is deleted. Its data shapes are [`contract::encounter`](../../../crates/contract/src/encounter.rs); the recipes are [`fixtures/encounters.json`](../../../fixtures/encounters.json), revision `encounters-1`, validated at load. The decisions the spec left open are in the [choices ledger](../choices.md#the-planner-checks-physical-placement-and-planned-reach-without-claiming-arrival-or-combat).

**The seam.**

```text
plan_encounter(&MapQueries, &Rules, &EncounterRecipe, encounter_seed: Seed)
  -> Result<EncounterDefinition, Vec<EncounterDiagnostic>>
```

- `MapQueries` is the compiled map, its `EncounterSites`, and the simulation's world, navigation grid and road graph over it, all borrowed. `PreparedMap::new(map, rules)` builds the three for a caller that has none; preparation that already holds a battle's world (C33) lends that one.
- `EncounterSites` is what a plan knows that the compiled map does not: each settlement's centre, outline and districts, and the measured open approaches. Generation hands it out beside the map (`GeneratedMap.sites`; `mapgen generate-map` saves `sites.json`).
- `EncounterRecipe` is intent: which side attacks from which edge, each side's roster with a post per row (`column`, `garrison`, `overwatch`), the objective preference, and the planner's distances and attempt limits. It holds no coordinates.
- `EncounterDefinition` is the recipe's hash, the encounter seed, `setup` (units, scripted orders, the opponent policy with its garrison rows, the capture zone: the scenario's own fields after its map and rules) and `placement` (the objective, each column with its drive, each garrison with its building and seats, each overwatch post, attempts: what an overlay or a report reads, and nothing the battle reads).
- `plan_encounter_json(map, sites, rules, recipe, seed)` is the same call over JSON and returns `{"status":"ok","encounter":…}` or `{"status":"error","diagnostics":[…]}`. The Wasm export `plan_encounter` is that function. The preparation worker calls `generate_map`, then `plan_encounter` with the generator's own map and sites text, and splices `setup` after the map and rules to make the scenario.

**The placement rules**, each asked of the world and navigation a battle runs on ([`legality`](../../../crates/sim/src/encounter/legality.rs)):

- A vehicle stands where its whole hull is on ground a mover may stand on (not water, not too steep), no body that stops vehicles overlaps it, and navigation's clearance has room for it. A squad stands where its middle is such ground and its spread holds a standing place for every soldier that he can walk to from the middle. Unit footprints keep the recipe's clearance from each other.
- Each side's column stands on a road that meets its own map edge, its tail an inset in from the edge and each unit a spacing ahead of the next, heading up the road. Of the roads at its edge it takes the one its pace unit (a jeep) drives soonest from. A column whose ground is not legal moves up its road a step at a time.
- The two drives to the objective must differ by no more than the recipe allows: the side with the longer drive starts further up its road, up to a limit.
- The objective is a settlement's centre, the capture zone a circle about it. Every unit type of a column, every garrison squad and every overwatch post must have a route navigation plans that ends inside the zone.
- A garrison squad takes a building within reach of the centre (the zone's own radius in the shipped recipe), in one of the settlement's districts, with a seat for every soldier, apart from the others chosen, and starts on legal ground outside one of its doors. One squad to a building. Buildings on the side of the centre the attack comes from are tried first; the encounter seed orders them within each side.
- An overwatch post stands just beyond the settlement's last ground on a way in: beside the road the attacker drives in by, then on the open approaches on the attacker's side. Of the places tried along that edge it takes the one that sees farthest down that way.
- A search that runs out is a diagnostic naming the recipe row or field: `no_objective` (with the reason for each settlement tried), `no_open_approach`, `no_edge_road`, `no_deployment`, `unreachable_objective`, `unfair_deployment`, `no_garrison_building`, `no_overwatch_post`, `invalid_recipe`, `invalid_sites`, `invalid_request`.

**One fixed seed per type and size** (`encounter_report`, recipe `assault`, encounter seed 1, native release build; five of the nine maps have a river):

| Map (type, size, map seed) | Buildings | River | Blue's drive | Red's drive | Difference | Column moved up (blue / red) | Garrisoned squads | Attempts | World build | Planning |
|---|---|---|---|---|---|---|---|---|---|---|
| Open Small 2 | 701 | yes | 106.6 s | 115.4 s | 8.8 s | 0 / 0 m | 3 | 9 | 12.7 G | 0.18 G |
| Mixed Small 1 | 3,055 | no | 88.9 s | 92.9 s | 4.0 s | 0 / 0 m | 3 | 9 | 5.5 G | 0.21 G |
| Metro Small 3 | 1,419 | yes | 92.7 s | 93.2 s | 0.5 s | 550 / 0 m | 3 | 12 | 10.3 G | 0.29 G |
| Open Medium 1 | 815 | yes | 120.0 s | 126.4 s | 6.5 s | 0 / 0 m | 3 | 9 | 22.4 G | 0.15 G |
| Mixed Medium 2 | 4,788 | yes | 122.1 s | 124.1 s | 2.0 s | 0 / 0 m | 3 | 9 | 24.5 G | 0.20 G |
| Metro Medium 2 | 5,674 | no | 118.4 s | 126.4 s | 8.0 s | 0 / 0 m | 3 | 11 | 8.7 G | 0.18 G |
| Open Large 1 | 1,615 | no | 158.4 s | 158.3 s | 0.2 s | 850 / 0 m | 3 | 15 | 6.0 G | 0.21 G |
| Mixed Large 5 | 7,200 | yes | 154.9 s | 161.1 s | 6.2 s | 0 / 0 m | 3 | 9 | 26.4 G | 0.26 G |
| Metro Large 7 | 12,811 | no | 149.4 s | 160.1 s | 10.7 s | 0 / 0 m | 3 | 9 | 10.1 G | 0.28 G |

Every cell plans; none is refused. A drive is the jeep's time from the head of a column to the objective, by navigation's fastest route. World build and planning are instructions retired: planning is the planner alone, the world build is the terrain, navigation grid and road graph it asks, which a battle builds again for itself today.

**Proof.**

- Each rule and each ruled-out moment: `crates/sim/tests/encounter.rs` (17 tests on a small authored town): a column whose road starts under water stands on the far bank or is refused, a column stands clear of a building on its road, an objective cut off by an unbridged river is refused, a squad one soldier larger than a building's seats is not garrisoned, more garrison rows than buildings in reach is refused, no road at an edge and no settlement are named refusals, the farther column moves up until the drives match or the start is refused as unfair, and a planned encounter starts a battle whose three garrisons are inside their buildings after 20 s while the relief column drives for the town.
- Native and Wasm plan the same bytes: `fixtures/parity/encounter/paired-records.json` holds six cases (three placements, one with an encounter seed above the JavaScript integer range, and three refusals), replayed by `crates/mapgen/tests/encounter.rs` and held by `web/tests/encounter.test.ts`.
- The overlay per cell is `encounter_report --out <directory>`: the whole map, the objective, each overwatch post and each column, as SVG.
- `/lab/generated` plays the planned encounter and its `generated` scene passes. Preparation (generating the map, building the planning world and planning) took 2.1 s for Mixed Small in the development build.

**Not proved here.** The C58 fixed-seed map does not exist yet, so the planner has not been run on it. Saved and runtime preparation yielding one replay identity is C55's. The multiple-seed matrix is C54's. No battle was played to show an engagement at 1,800 m: the recipe requires an open approach at the objective and a second overwatch row covers it, but the shipped roster has one overwatch row and it watches the attacker's road. The overlays were reviewed by the implementer and by one unprimed critique, which changed three things (the garrison holds the attacker's side of the zone, posts stand on the settlement's edge and not out in the field beside it, the jeep leads red's column) and left two standing: an overwatch post can stand on open ground with no cover, and blue's squads start on foot kilometres from the objective. `compare-screenshots` and `preview-shots` were not run.
