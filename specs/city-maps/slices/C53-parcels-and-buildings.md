# C53: template-aware parcels and buildings

**Depends on:** C52, C04, C13's reusable geometry catalogue. **Kind:** required slice.

## Question
Can varied legal building templates fill settlement plots while keeping streets, entrances and battle routes open?

## Contract it unlocks
`fill_districts(MapPlan, GenerationRequest, TemplateGeometryCatalog) -> Result<MapPlan, Vec<Diagnostic>>` cuts blocks/parcels and selects legal immutable templates using G0's type weights and one regional family per map. The six categories and floor exclusions are defined in the completed map. Open never selects seven-storey+ buildings; highrises are Metro-only. Class labels do not change Q3/Q4 combat rules.

A template's physical footprint, parts, floor heights, entrances and join capabilities constrain parcel fill. Fit parcels to legal envelopes or choose another eligible template; never stretch houses, floors or streets to fill a lot. Translate/rotate at stable metre scale. Courts/yards are intentional parcel results. Entrances face reachable streets and compounds/terraces use only S5/C13's baked compatible edge variants. Unsupported fits return bounded diagnostics.

C04 materializes authoritative building facts from the selection. C13 is a reusable input built independently of maps; there is no per-generated-part Blender job. Map/request identity pins the physical catalogue. C32 separately pins compatible appearance; replacing materials/LOD never changes this map.

## API seam
`mapgen::parcels` → C04 MapPlan rows → C01 MapDefinition.buildings. C13 supplies C00 geometry descriptors; C32 and C22 resolve shared module placements. The renderer never infers lots from textures.

## What the human can run or see
Workbench layers for parcels, category/floors, selected template ID, entrances and rejected fits. One generated block renders from the reusable library without rerunning Blender.

## Verification
- Canonical seed/request/physical-catalogue identity and stable building IDs; complete native/wasm parity against frozen S6 inputs.
- Descriptor bounds/floors match compiled geometry; no overlap with roads, rivers, reserved plains or other buildings beyond G0's named tolerances.
- Every entrance reaches a street by infantry; vehicles can cross town into plain.
- All type × size cases use one compatible regional family, satisfy exclusions and stay within full-extent budgets.
- Visual variable: parcel fit and building distribution only. Compare overlays and block shots with accepted layout and template evidence using compare-screenshots; run unprimed screenshot-critique last; preview-shots non-blocking.

## Delegated to the implementer
Parcel subdivision and eligible within-preset template variation. Template stretching, ad hoc art fallback and new weights are not delegated.

## Must stay green
One authoritative building row, matching template art and accessible streets/plains.

## Feedback that would change this slice
Insufficient variety or poor street frontage expands the template library/preset evidence rather than inventing per-map art.
