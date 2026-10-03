# C04: map plan compiler

**Depends on:** C01, C03, C69, C72; C65 through C69; G0's architecture/identity verdicts. **Kind:** slice.

## Historical pass 1: physical building and identity seam

The independently owned building/compiler core is implemented in
[`crates/mapgen`](../../../crates/mapgen/README.md). It resolves catalogue placements
through C00/C01's final physical owner, shares exact seed/content identity with native
and WASM callers, and refuses requested features whose shared admission is not ready.
Caller limits bound authored parts and cumulative bay coordinates before geometry
materialization; these limits are not an all-world capacity proof.

The [focused evidence](../assets/map-compiler/README.md) retains complete CLI/WASM
records and original physical oracles. At this checkpoint, shared surface admission, rivers, forests,
overlap/composition validation and inspection were later compiler arms. C09/C13/C32 own source provenance, selection readiness and appearance
fit. This pass does not close the whole C04 or G0 source/art gate.

## Historical pass 2: roads and forests

`MapPlan.surfaces` and `MapPlan.forests` lower into the map in the contract's own
shapes (C03, C64, C65, C72). Authored points must lie inside the playable bounds, and
`limits.max_ground_points` bounds polygon vertices plus rounded stroke samples;
`report.ground_points` states what was used. Native and Wasm agree on the complete
records, including one accepted and two refused ground plans
(`fixtures/parity/map-compiler/paired-records.json`). At this checkpoint, rivers (C69), overlap/composition validation and inspection remained.

## Question
Can one typed plan become authoritative compiled geometry and identity without a second interpretation by the battle or renderer?

## Contract it unlocks
The [Rust library/CLI](../../../crates/mapgen/README.md) depends on `contract`, not scene assets. `MapPlan` carries bounded physical placements, surfaces, rivers, forests and props, plus generation-only settlement, district, parcel and approach data. The compiler materializes selected template geometry into the contract’s building parts, floors, heights and entrances. Lowering emits the shared physical `MapDefinition`, including playable bounds and the separate visual margin, together with generation identity and diagnostics. Settlement metadata grants no gameplay properties and never reaches the renderer.

Urban yards and agricultural surroundings use [C31’s accepted renderer plot owner](C31-city-biome.md#outcome), derived from exported buildings and physical roads. No `MapDefinition.land_regions` schema was admitted; requesting that unsupported plan field is explicitly refused. C54 checks physical construction and playability separately from visual crop/yard composition.
`GeneratedMap` contains the compiled definition, generation identity and diagnostics/report. Identity pins generator version, preset revision, canonical config, lossless seed, physical template catalogue hash and map hash; appearance identity is separate presentation metadata. The CLI writes saved `fixtures/maps/<id>/{map.json,SOURCES.json}`; runtime returns equivalent identity in memory/replay. C09 owns acquisition and C60 owns saved-map catalogue metadata.

## API seam
The compiler library’s typed plan and result → `contract::map::MapDefinition` and shared identity types. Generator plan/debug overlays never cross into the sim or renderer. C65 owns shared curve densification; C69/C72 establish river/forest schema before this compiler.

## What the human can run or see
`mapgen inspect <plan>` renders plots, roads, parcels, legal template footprints, entrances and plains with labelled diagnostics. A small hand-authored plan using a prototype descriptor is sufficient; production art is not required here.

## Verification
- Golden canonical bytes/hash against frozen S6 input, with only named schema differences; invalid geometry yields stable diagnostics.
- Compiled building geometry equals transformed descriptor geometry; template IDs resolve to the pinned physical catalogue and unsupported fit is rejected. Art coverage/fit belongs to C32.
- Roads/rivers/forests share the authoritative physical geometry; no renderer smoothing or generator-only route answers.
- Village parity here; intentional surrounding changes belong to C56.
- Compare the overlay against expected layers using compare-screenshots, then run unprimed screenshot-critique last; preview-shots is a non-blocking checkpoint.

## Delegated to the implementer
Polygon clipping and indexing algorithms within G0's bounds; internal names. Representation/identity changes beyond G0 are spec gaps for choices.md.

## Must stay green
One compiled map contract and one geometry owner; no runtime generated-map behavior branch.

## Feedback that would change this slice
A frozen spike parity failure reslices the affected seam before generator consumers land.

## Current outcome

Buildings and the shared road, forest, river, bridge and prop shapes compile through one physical owner. The [generator README](../../../crates/mapgen/README.md) owns current admission, limits, canonical identity and CLI tooling. Plan-only metadata supports layout measurement, inspection and encounter sites; C31 owns the visual plot interpretation. Historical pass reports above establish their original scopes and are not the current pending implementation list. Source/art fit and full-world release admission remain separate gates.
