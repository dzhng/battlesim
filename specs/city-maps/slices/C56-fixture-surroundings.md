# C56: focused-arena composition contract

**Depends on:** C53, C09 and C60. **Kind:** required slice.

## Question
Can one reservation contract add physical surroundings without changing a focused arena's protected behavior?

## Contract it unlocks
Inventory every shipped/catalogued world through fixtures, routes and benchmark builders, including endurance and synthetic benchmark worlds. Do not assume the historical map count is complete. Pure low-level geometry probes are API test inputs; composition is validated at the catalogue/map-preparation boundary.

A typed `ArenaReservation` declares protected physical geometry, an exclusion envelope and measurement bounds. The same Rust mapgen composition owner adds deterministic urban templates, roads and usable flat plains outside that reservation, then C04 compiles the result. Diagnostic maps keep purpose-appropriate bounds; the player size presets describe primary generated battle maps and do not force each lab onto a full-size battlefield.

Preserve protected geometry, test stimuli, units/orders, objectives and frozen camera stations. Prefer placements outside the arena without translation; if bounds require translation, one composition step transforms all coordinates together and names the changed expectations. Surroundings are inside sim bounds and use normal physical bodies. No renderer-only town silhouette or geometry ignored by sight/nav satisfies the rule.

This slice proves the seam on geometry-lab; C34 migrates the village, C35 remaining labs and C36 benchmarks. Each migration declares protected behavior probes and purpose-appropriate exclusion bounds before composing. No universal sensor-range padding is prescribed: a margin alone cannot prove no new route/sight interaction. Register sources/config/hashes in the existing map provenance. Whole-map identities/digests and frame costs will change where geometry is added; name them, prove arena behavior first, then rebaseline affected reports. Do not promise blanket digest parity for this migration or hide allocation/render overhead outside a measurement crop.

## API seam
`mapgen::compose_arena(ArenaReservation, surroundings inputs) → MapPlan` through C04; existing fixture/catalogue builders consume the compiled map. C60 owns metadata; there is no diagnostic-only runtime generator or second geometry format.

## What the human can run or see
An inventory report, before/after arena and whole-map overlays, geometry-lab still serving its focused scenario with real town/plain surroundings. The inventory assigns every other producer to C34, C35 or C36.

## Verification
- Inventory cross-check: every map-producing route/catalogue/benchmark source is accounted for.
- Protected geometry, scripted orders and arena mechanics still satisfy original focused assertions; surroundings do not encroach or introduce unwanted sight/route interactions.
- Real urban/plain geometry and physical infantry/vehicle access outside the arena; original scenario names remain usable.
- Named map/config/digest changes and rebuilt replay expectations; aggregate-only parity claims from C01/C09/C60 do not apply to this geometry addition.
- Full-world startup/memory and whole-frame cost are measured against migrated baselines, including surroundings.
- Compare arena masks and full-map composition using compare-screenshots; run unprimed screenshot-critique last; preview-shots non-blocking.

## Delegated to the implementer
Geometry-lab reservation bounds/minimal layout, justified by its protected probes and recorded in choices.md. Other migration slices own their arena-specific bounds. Changes to the arena's intended behavior are not delegated.

## Must stay green
Focused scenario contracts, all-map physical composition and one mapgen/compiler/resolver path.

## Feedback that would change this slice
An arena interaction forces a different reservation/layout; it cannot waive real surroundings or silently weaken a test.
