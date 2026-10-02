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

The library, its loader path, the resolver and the fit and coverage checks are in `packages/scene-assets` ([its readme](../../../packages/scene-assets/README.md), "City buildings"), and the bake produces a real library today from the generated prototype set: the 29 prototype templates, 49 rows of one module. The contract's own code judges every descriptor and the catalogue's hash, through one new WebAssembly export. The decisions are in [choices](../choices.md#c13c32-kits-and-the-template-art-library).

**Not done here.** Nothing draws the library yet: placement chunks (C22) call the resolver, and the massing boxes go then. The contact sheet this slice names waits for that, since a kit in the workbench is its modules, not a building. `ruin` and `gutted` rows pack and resolve but have no fit rule of their own yet (C14): they are held to the intact parts. The per-template triangle budgets are reported by the bake and gate nothing.
