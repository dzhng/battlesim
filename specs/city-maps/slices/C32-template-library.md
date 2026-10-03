# C32: template appearance library and resolver

**Depends on:** C13, C00. **Kind:** slice.

## Question
Can saved and runtime maps resolve the same physical template to bounded shared-module placements through one asset contract?

## Contract it unlocks
`scene-assets` deterministically packs exported sources into `TemplateArtLibrary { art_hash, covers: template_catalog_hash, templates }`. Each template/state row contains shared module placements and `prototype|release` status. C14 adds terminal rows through the same builder. Art identity includes module/material/placement/LOD content and the covered physical identity; it never enters simulation map JSON.

`resolve(template_id, placed_frame, side_known_state, library) → placements` is pure and feeds C22. Asset check re-encodes/validates exported sources without launching Blender. Fit validation compares placements to C00 geometry and the resolved terminal rules. Missing templates/modules/state rows fail explicitly, with no cross-region fallback. Library files follow the existing live asset bake/cleanup owner.

Coverage is incremental: a workbench may resolve explicit prototype rows; a playable review uses release rows for every template it actually selects. C54 requires release art for every category/family selectable by final presets. G0 settles the supported family matrix; this slice does not invent additional regional requirements.

## API seam
Existing scene-assets bake/schema/loader → deterministic library codec + pure resolver → C22 static chunks. Physical catalogue and appearance hashes are separate.

## What the human can run or see
A compact template/state contact sheet and byte report; render the same selected templates from saved and runtime definitions without a source export.

## Verification
- Cold/warm packing byte identity and saved/runtime resolver parity.
- Prototype-to-release or material/LOD changes leave map/scenario/battle identity unchanged.
- Physical catalogue changes invalidate incompatible coverage; missing rows/modules and failed fit have stable diagnostics.
- Placement decoding and bounded resident expansion meet G0's limits.
- Judge assembly/frame/fit only; fine materials and whole-map composition are later variables. Compare with C13 fit overlays using compare-screenshots; run unprimed screenshot-critique last; preview-shots non-blocking.

## Delegated to the implementer
Codec grouping/packing within G0's choice; new geometry and release matrix are not delegated.

## Must stay green
One asset lifecycle, matching geometry, shared instancing and art-independent simulation identity.

## Feedback that would change this slice
Packing or resolver failures reopen this codec; missing art reopens its source slice.

## Outcome

The library, its loader path, the resolver and the fit and coverage checks are in `packages/scene-assets` ([its readme](../../../packages/scene-assets/README.md), "City buildings"), and the bake produces a real library today from the generated prototype set: the 29 prototype templates, 49 rows of one module. The contract's own code judges every descriptor and the catalogue's hash, through one new WebAssembly export. The decisions are in [choices](../choices.md).

**Not done here.** Nothing draws the library yet: placement chunks (C22) call the resolver, and the massing boxes go then. The contact sheet this slice names waits for that, since a kit in the workbench is its modules, not a building. `ruin` and `gutted` rows pack and resolve but have no fit rule of their own yet (C14): they are held to the intact parts. The per-template triangle budgets are reported by the bake and gate nothing.

**Kits on request (2026-10-02).** The library is still installed whole with the catalog, but its kits no longer are: a kit is fetched when a map that draws from it asks, and the library is held to the catalog's kit hashes at load without fetching one. A module whose kit is absent is bound to no state, and a map whose kit is not installed is refused by name. The village battle fetches 38.8 MB of kits where every page fetched 116.3 MB; the decisions and the measurements are in [choices](../choices.md).

## Lossless transport checkpoint

Kits and the template library now travel as required gzip objects through the existing appearance loader and baker. Encoded files have their own content addresses; decoded art, templates, physical catalog and source GLBs retain their exact identities. The [asset owner](../../../packages/scene-assets/README.md) defines validation and loading. Both browser and native selection admission count the library plus unique requested kit objects before reading dependent payloads; a catalog reload retries the complete original selection.

The real Market Town battle's selected transfer is 24,252,882 bytes (23.13 MiB), down from 95,027,440 bytes (90.63 MiB). All five generated kits plus library total 27.64 MiB, below the unchanged 50 MiB shared-download gate. The unchanged decoded per-kit limit and resident/GPU/startup budgets remain separate. All nine migrated objects decode to the original bytes; old raw runtime duplicates are removed, and identified unit/scenery/skeleton consumers retain their raw inputs.

Focused loading, integrity, overflow, admission and reload tests pass; independent review verifies every encoded and decoded identity. A fresh browser smoke on matching native/Wasm source loads actual Market Town without errors. Retained evidence and choices are in the main checkout's ignored `throwaway/city-kit-transport/`. The controlled cold raw/gzip comparison uses its original pre-weapons engine: comparable readiness, +0.45% sampled retired-instruction lower bound and +2.83% charged footprint in one pair. It is transport evidence, not current whole-system memory or startup acceptance.
