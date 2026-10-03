# C19: warehouse and light-industry silhouettes

**Depends on:** C00 descriptor schema, C11 module export, C12 materials and S2/G0's recipe matrix. **Kind:** slice.

## Question
Does the warehouse/light industry family have a believable silhouette at stable metre scale within the chosen regional families?

## Contract it unlocks
A project-owned authored family source using dedicated industrial geometry, rather than enlarged houses. Legal recipes cover G0 recipe-defined floors and wide industrial sheds, yards and light-industry frontage. They emit the same module inventory/local placement rows and physical descriptor inputs proven by S2, ready for C13 source export and C32 library packing. G0 fixes recipe dimensions/variety and regional eligibility; the source does not introduce a per-map art path.

Reuse compatible masonry/roof/window primitives and module export ownership. Keep physical envelope, floor heights, entrances and facade bay patterns aligned with the descriptor. Regional variants belong to one coherent family per map; missing variants cannot fall back to unrelated regional art. C14 owns damage states after the intact library exists.

## API seam
Authored family recipe → existing scene-assets module export + contract-typed geometry input → C13.

## What the human can run or see
Neutral-material silhouette/contact sheets at ground, 80 m, 250 m and the far transition, plus physical fit overlays. Compare several legal variants at the same camera/scale.

## Verification
- Deterministic source/output identity and physical footprint/height/floor fit.
- Recipe coverage/variety and triangle/module budgets from G0; larger map sizes do not change source scale.
- Visual variable: warehouse/light industry massing/silhouette only. Materials, street placement and damage are separate gates.
- Use compare-screenshots against G0's frozen recipe/reference proportions and matched variant sheets. Run unprimed screenshot-critique last; ask whether the silhouette reads as warehouse/light industry. Use preview-shots for non-blocking review and record the decision.
- Apply renderer before authored asset changes; missing reference inputs remain explicit rather than invented as approved art.

## Delegated to the implementer
Roof/facade detail and bounded cosmetic variants within the approved dimensions/family. New class semantics, region mixing and physical scaling are not delegated.

## Must stay green
Shared module/library ownership, physical fit and the category's metre scale.

## Feedback that would change this slice
Rejected silhouette reshapes its family source without changing the type preset or adding a per-map bake.

## Outcome

**2026-10-01.** Industry is a source set: `packages/scene-assets/blender/city/industry.py` writes `assets/source/city/industry/` (31 modules, five templates), through `city/kit.py`. The five are a steel workshop shed (15 x 24 m, 6.2 m), a precast dock warehouse (48 x 24, 9 m), a twin-span steel warehouse (72 x 33, 10 m), a brick works with a sawtooth roof and a joined office block (54 x 36 overall, 8 m, two floors) and a blockwork depot with a roof monitor (90 x 39, 11 m). The sheets this slice asks for were Blender renders from a script of their own, since deleted; the line-up lab (`/lab/city-lineup`) is where the set is judged now.

- **Proved:** two runs write the same bytes, and the houses' and farmsteads' scripts still write theirs. Every descriptor meets the generator's demands (one or two floors, 24 to 96 m, doors on the street side with 60 m clear, checked by a scratch script that repeats the catalogue test's rules). The kit validates with no finding; the bake's only findings for the set are that its templates are not yet in the physical catalogue, which the cutover clears.
- **Budget:** a template draws 5,170 to 20,580 triangles at tier 0, 2,260 to 8,640 at tier 1, 440 to 1,920 at tier 2 and 56 to 224 at tier 3, in 68 to 276 rows at the fine tiers and one row at the coarse ones.
- **Seen:** an unprimed critique named all five correctly from the pictures (workshop shed, loading-dock warehouse, brick factory with a north-light roof, steel storage shed, vehicle depot) and found the five roofs distinct at 250 m. What it faulted and is still true: the depot and the big warehouse share a wall vocabulary (a high window band, vehicle doors under canopies); the depot's red-brown roof is close to the houses' tiles; rooftop plant is plain boxes; a far building (tier 3) loses its windows.
- **Not done:** the sheets are Blender renders with a stand-in sun, not the battle's renderer. No comparison against the reference was scored. Damage states are C14's. Whether garrison seats in blank bays and at loading docks play well is untested.
- **Decisions:** [choices](../choices.md).
