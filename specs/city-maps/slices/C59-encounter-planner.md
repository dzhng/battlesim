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
