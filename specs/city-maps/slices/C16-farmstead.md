# C16: farmstead silhouettes

**Depends on:** C00 descriptor schema, C11 module export, C12 materials and S2/G0's recipe matrix. **Kind:** slice.

## Question
Does the farmstead family have a believable silhouette at stable metre scale within the chosen regional families?

## Contract it unlocks
A project-owned authored family source using the existing house.py courtyard-farm source. Legal recipes cover 1–2 floors and courtyard farm buildings and rural dwelling wings. They emit the same module inventory/local placement rows and physical descriptor inputs proven by S2, ready for C13 source export and C32 library packing. G0 fixes recipe dimensions/variety and regional eligibility; the source does not introduce a per-map art path.

Reuse compatible masonry/roof/window primitives and module export ownership. Keep physical envelope, floor heights, entrances and facade bay patterns aligned with the descriptor. Regional variants belong to one coherent family per map; missing variants cannot fall back to unrelated regional art. C14 owns damage states after the intact library exists.

## API seam
Authored family recipe → existing scene-assets module export + contract-typed geometry input → C13.

## What the human can run or see
Neutral-material silhouette/contact sheets at ground, 80 m, 250 m and the far transition, plus physical fit overlays. Compare several legal variants at the same camera/scale.

## Verification
- Deterministic source/output identity and physical footprint/height/floor fit.
- Recipe coverage/variety and triangle/module budgets from G0; larger map sizes do not change source scale.
- Visual variable: farmstead massing/silhouette only. Materials, street placement and damage are separate gates.
- Use compare-screenshots against G0's frozen recipe/reference proportions and matched variant sheets. Run unprimed screenshot-critique last; ask whether the silhouette reads as farmstead. Use preview-shots for non-blocking review and record the decision.
- Apply renderer before authored asset changes; missing reference inputs remain explicit rather than invented as approved art.

## Delegated to the implementer
Roof/facade detail and bounded cosmetic variants within the approved dimensions/family. New class semantics, region mixing and physical scaling are not delegated.

## Must stay green
Shared module/library ownership, physical fit and the category's metre scale.

## Feedback that would change this slice
Rejected silhouette reshapes its family source without changing the type preset or adding a per-map bake.

## Outcome

**2026-10-01.** The farmsteads are a source set: `packages/scene-assets/blender/city/farmsteads.py` writes `assets/source/city/farmsteads/` (20 modules, three templates) through the houses' helper `city/kit.py`. The three farms replace the prototype shapes `prototype-farmstead-yard`, `-long` and `-small` at the catalogue cutover, which this slice did not run.

- **The farms:** `china-farmstead-yard` (a plastered two-storey house, a timber-framed barn with wagon doors, an open-fronted cart shed, round a yard; 34 x 24.5 m), `china-farmstead-long` (a whitewashed longhouse and a 26 m stone byre behind it; 26 x 27 m), `china-farmstead-small` (a brick house beside a tarred boarded barn; 26.5 x 14 m). Each building is a part of its own, standing apart; the yard is open ground.
- **Proved:** two runs write the same bytes; `homes.py` writes the same bytes as before its helpers were lifted into `kit.py` and `masonry.py`; the kit validates with no finding; the bake admits all three descriptors as complete and finds every row inside its parts, leaving only `templates.catalogue` (not in the physical catalogue), which the cutover clears. Every entrance is on the street side with a clear 60 m out of the door.
- **Budget:** triangles drawn at tiers 0 to 3: yard 13,416 / 5,016 / 666 / 74; long 8,832 / 3,254 / 338 / 44; small 7,824 / 3,018 / 282 / 36. At tiers 2 and 3 a farm is one row.
- **Judged by looking:** reassembly sheets (`city/assemble.py`, which gained a `hamlet` sheet) at 30, 80 and 250 m, the three in a row at 80 m and a hamlet of nine from 250 m. An unprimed critique of them named each group a farm with a house and a barn at 30 and 80 m with high confidence, and by roof colour and footprint only at 250 m. Its findings that were acted on: the barn windows' pale frames, the long house's plain chimneys, the brick plinth's white blotches, the small farm's 3.5 m gap, the framing's crawl at 250 m. Left: the roof recipe's lattice of tone and the pink and teal in its slate (the houses share the recipe), and no yard surface, wall or gate (the ground is the map's).
- **Not done:** the sheets are Blender renders with a stand-in sun, not the battle's renderer. No comparison against a reference image: the spec's references (`specs/city-maps/assets/reference/`) hold towns and ground, no farm. Damage states are C14's.
- **Decisions:** [choices](../choices.md).
