# C04: map plan compiler

**Depends on:** C01, C03, C69, C72; C65 through C69; G0's architecture/identity verdicts. **Kind:** slice.

## Pass 1: physical building and identity seam

The independently owned building/compiler core is implemented in
[`crates/mapgen`](../../../crates/mapgen/README.md). It resolves catalogue placements
through C00/C01's final physical owner, shares exact seed/content identity with native
and WASM callers, and refuses requested features whose shared admission is not ready.
Caller limits bound authored parts and cumulative bay coordinates before geometry
materialization; these limits are not an all-world capacity proof.

The [focused evidence](../assets/map-compiler/README.md) retains complete CLI/WASM
records and original physical oracles. Shared surface admission, rivers, forests,
land regions, overlap/composition validation and the inspect overlay remain later
compiler arms. C09/C13/C32 own source provenance, selection readiness and appearance
fit. This pass does not close the whole C04 or G0 source/art gate.

## Pass 2: roads and forests

`MapPlan.surfaces` and `MapPlan.forests` lower into the map in the contract's own
shapes (C03, C64, C65, C72). Authored points must lie inside the playable bounds, and
`limits.max_ground_points` bounds polygon vertices plus rounded stroke samples;
`report.ground_points` states what was used. Native and Wasm agree on the complete
records, including one accepted and two refused ground plans
(`fixtures/parity/map-compiler/paired-records.json`). Rivers (C69), land regions,
overlap/composition validation and the inspect overlay remain.

## Question
Can one typed plan become authoritative compiled geometry and identity without a second interpretation by the battle or renderer?

## Contract it unlocks
`crates/mapgen` is a Rust library/CLI depending on `contract`, not scene-assets. `MapPlan` has metre bounds, plot polygons, road/river control runs, forests, plain polygons, blocks/parcels and selected building-template placements. Template geometry has the contract-owned descriptor shape from C00/C13. Only C04 materializes transformed building parts, floors/heights, entrances and exposed edges into `MapDefinition`; no arbitrary per-map building bake follows.

`validate_plan(&MapPlan) -> Result<(), Vec<Diagnostic>>` reports invalid rings/bounds, overlap, missing templates, unsupported joins and bounded-complexity failures with stable feature/location identities. `lower(&MapPlan, &TemplateGeometryCatalog) -> Result<GeneratedMap, Vec<Diagnostic>>` writes the contract's buildings, surfaces, rivers, forests and props. `MapDefinition.land_regions` holds `Urban | Plain` polygon rings: C31 reads them for composition, and C54 measures them. Region labels do not grant gameplay buffs.

`GeneratedMap` contains the compiled definition, generation identity and diagnostics/report. Identity pins generator version, preset revision, canonical config, lossless seed, physical template catalogue hash and map hash; appearance identity is separate presentation metadata. The CLI writes saved `fixtures/maps/<id>/{map.json,SOURCES.json}`; runtime returns equivalent identity in memory/replay. C09 owns acquisition and C60 owns saved-map catalogue metadata.

## API seam
`crates/mapgen::{MapPlan, validate_plan, lower, GeneratedMap}` → `contract::map::MapDefinition` and shared identity types. Generator plan/debug overlays never cross into the sim or renderer. C65 owns shared curve densification; C69/C72 establish river/forest schema before this compiler.

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
