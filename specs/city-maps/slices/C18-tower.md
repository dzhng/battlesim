# C18: highrise silhouettes

**Depends on:** C00 descriptor schema, C11 module export, C12 materials and S2/G0's recipe matrix. **Kind:** slice.

## Question
Does the highrise family have a believable silhouette at stable metre scale within the chosen regional families?

## Contract it unlocks
A project-owned authored family source using a dedicated tower source; existing corner-apartment graphs do not establish tower coverage. Legal recipes cover 9+ floors and a true tall tower and its readable base/roof massing. They emit the same module inventory/local placement rows and physical descriptor inputs proven by S2, ready for C13 source export and C32 library packing. G0 fixes recipe dimensions/variety and regional eligibility; the source does not introduce a per-map art path.

Reuse compatible masonry/roof/window primitives and module export ownership. Keep physical envelope, floor heights, entrances and facade bay patterns aligned with the descriptor. Regional variants belong to one coherent family per map; missing variants cannot fall back to unrelated regional art. C14 owns damage states after the intact library exists.

## API seam
Authored family recipe → existing scene-assets module export + contract-typed geometry input → C13.

## What the human can run or see
Neutral-material silhouette/contact sheets at ground, 80 m, 250 m and the far transition, plus physical fit overlays. Compare several legal variants at the same camera/scale.

## Verification
- Deterministic source/output identity and physical footprint/height/floor fit.
- Recipe coverage/variety and triangle/module budgets from G0; larger map sizes do not change source scale.
- Visual variable: highrise massing/silhouette only. Materials, street placement and damage are separate gates.
- Use compare-screenshots against G0's frozen recipe/reference proportions and matched variant sheets. Run unprimed screenshot-critique last; ask whether the silhouette reads as highrise. Use preview-shots for non-blocking review and record the decision.
- Apply renderer before authored asset changes; missing reference inputs remain explicit rather than invented as approved art.

## Delegated to the implementer
Roof/facade detail and bounded cosmetic variants within the approved dimensions/family. New class semantics, region mixing and physical scaling are not delegated.

## Must stay green
Shared module/library ownership, physical fit and the category's metre scale.

## Feedback that would change this slice
Rejected silhouette reshapes its family source without changing the type preset or adding a per-map bake.

## Outcome

**2026-10-01.** The towers are a source set: `packages/scene-assets/blender/city/towers.py` writes `assets/source/city/towers/` (30 modules, four templates) through the houses' authoring helper. `city/tower_sheets.py` photographs them with `assemble.py`'s camera, alone and among the houses and the China apartment blocks.

| Template | Plan, floors, height | Look |
|---|---|---|
| `china-tower-slab-10f` | 56 × 14 m, 10, 31 m | bare grey precast, three stairs, paired balcony columns in faded paint, blank gable ends |
| `china-tower-12f` | 26 × 26 m, 12, 37 m | cream render, loggias, a terracotta stair stripe |
| `china-tower-16f` | 23 × 32 m, 16, 49 m | white facing tile, aqua balcony columns every third bay |
| `china-tower-20f` | 29 × 29 m, 20, 61 m | pale precast, corner loggias, slate-blue stair and blank columns |

- **How a tower is drawn:** at tier 0 every bay of every floor is a row placing one shared panel module (40 triangles for a window). From tier 1 the panels stop and the template's shell carries the same grid as a texture. At tiers 2 and 3 a tower is that one row.
- **Budget, triangles drawn at tiers 0 to 3:** slab 27,022 / 7,510 / 918 / 334 (721 rows at tier 0); 12 floors 23,096 / 1,328 / 464 / 218 (564); 16 floors 37,396 / 11,810 / 726 / 396 (945); 20 floors 41,710 / 2,314 / 704 / 254 (1,044).
- **Proved:** two runs write the same bytes; the kit validates with no finding; the bake's only finding is that the four templates are not yet in the physical catalogue, which the cutover clears (so the contract admits the descriptors and every row stays inside its part grown by the fit). `homes.py` still writes its committed bytes.
- **Not done:** the sheets are Blender renders with a stand-in sun, not the battle's renderer, which cannot draw kits yet; the tier distances are `assemble.py`'s reading of the model thresholds. No reference comparison was run. Damage states are C14's.
- **Decisions:** [choices](../choices.md#c18-our-own-towers).
