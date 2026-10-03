# C45: street models

**Depends on:** C44, C11, C74. **Kind:** slice.

## Question
Does each prop's model fit its body?

## Contract it unlocks
Appearances:
- theirs, from C11's standalone street kit;
- ours: cars, wrecks, Jersey barrier, bus shelter, scaffold, Heras fence panel, skip bin, pallet stack, site cabin, traffic cone and road barrier, generic with no brands or plates. **Project-owned Blender models only:** the threejsassets.com Construction Site Kit (https://threejsassets.com/packs/construction-site) inspired the construction kinds, but its licence forbids redistribution and our repo is public, so none of its files may enter the repo;
- street trees from the one tree generator (C73/C74 species).

Each has LODs and its terminal state.

## API seam
`packages/scene-assets` appearances.

## What the human can run or see
A contact sheet per kind.

## Verification
- `asset check`; fit validation.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**body/model fit only; for the construction kinds also silhouette and read at 65 m**) against **the kind's body box overlay; for construction-site kinds, also the matching thumbnail in `specs/city-maps/assets/reference/threejsassets-construction/` (reference only, see its README)**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Placement density.


## Delegated to the implementer
Model details within 'generic'. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
C44's body values.

## Feedback that would change this slice
Street models that disagree with their body bounds change appearance fit while keeping physical rules fixed.


## Outcome (2026-10-02)

**Changed since.** [C46](C46-street-placement.md), merged alongside: generated maps place these props, at the preset's boxes and not yet at the models'; and a street tree is its own prop type, `street_tree`, which the scenery now draws as a tree ("Street trees" below).

**Contract as landed.** Every street row of `fixtures/props/city/street.json` is drawn by its own scenery kind (`SCENERY_KINDS`, one `default` state) with one project-owned appearance under `assets/source/street/`, built by `packages/scene-assets/blender/street.py` (seventeen kinds) and `street_car.py` (the car, and its wreck with `--wreck`). No row is `systems_only` any more; no body column changed, and the resolved `fixtures/catalog.json` is current (`catalog::the_browsers_catalog_view_is_current`). `asset check` passes and every appearance fits its box at the catalog's default 0.1 m tolerance, with no per-entry allowance.

**What differs from the slice as written.**
- The rows carry no size, so the box each model fits is the appearance's own `footprint_half_m` (below); C46 should place these sizes.
- "Theirs, from C11's standalone street kit" was not done: this lane made its own model for all nineteen kinds.
- Street trees got no new art: they are the existing tree body and species ("Street trees" below).
- The only terminal state is the car's, the `car_wreck` row's own appearance; every other row is `removed`.
- Jersey barrier, Heras panel and scaffold are `modular` (they repeat along a long box).

| Kind | Box (full, m) | Triangles per tier |
|---|---|---|
| parked_car | 4.4 × 1.8 × 1.44 | 6,008 / 2,024 / 572 / 300 |
| car_wreck | 4.4 × 1.8 × 0.7 | 4,804 / 1,384 / 380 / 92 |
| jersey_barrier | 3.0 × 0.6 × 0.8 | 468 / 168 / 44 / 28 |
| bollard | 0.2 × 0.2 × 0.9 | 400 / 184 / 80 / 40 |
| lamp | 0.4 × 0.4 × 5.0 | 696 / 266 / 112 / 80 |
| bench | 1.8 × 0.6 × 0.84 | 1,372 / 516 / 96 / 72 |
| bins | 1.3 × 0.76 × 1.1 | 816 / 320 / 152 / 48 |
| hydrant | 0.4 × 0.4 × 0.76 | 688 / 352 / 120 / 100 |
| utility_box | 1.4 × 0.5 × 1.4 | 928 / 464 / 60 / 36 |
| planter | 2.0 × 0.9 × 1.0 | 672 / 180 / 124 / 112 |
| bus_shelter | 4.0 × 1.5 × 2.5 | 2,412 / 908 / 312 / 156 |
| heras_fence | 3.5 × 0.6 × 2.0 | 2,176 / 662 / 160 / 72 |
| skip_bin | 3.6 × 1.7 × 1.2 | 2,080 / 712 / 180 / 72 |
| pallet_stack | 1.2 × 1.0 × 1.2 | 2,080 / 640 / 108 / 12 |
| site_cabin | 6.0 × 2.4 × 2.6 | 4,352 / 1,288 / 284 / 60 |
| traffic_cone | 0.4 × 0.4 × 0.74 | 216 / 88 / 44 / 32 |
| road_barrier | 2.0 × 0.5 × 1.0 | 720 / 296 / 120 / 72 |
| scaffold | 2.5 × 1.0 × 4.0 | 2,472 / 922 / 372 / 156 |
| scooter | 1.8 × 0.7 × 1.1 | 1,564 / 644 / 200 / 108 |

**Compare.** Fit was judged on the workbench sheet's own box overlay: every model fills its box and none passes it. Against the Construction Site Kit thumbnails (looked at after modelling, for the Heras panel, the skip and the scaffold): the same class of silhouette, in our own proportions and detail (a yellow skip heaped with spoil against their open orange one; a two-lift bay against their outrigged tower). Less wrong than a box; not a match, on purpose.

**Critique** (one unprimed reader per group, on the first sheets). No shadow read as fog. Dark faces that could read as shadow were the wreck's sooted sides, the barrier's streaked long face, the cabinet's wide faces and the shelter's dark panes: all four were lightened. Other findings and what was done:

- Fixed in one round: the car's lamps did not show (a dark band now carries them) and its rims were rust-orange; the barrier's foot dirt read as a lumpy flare and its lifting eyes as stray blocks (removed); the bins' lids hovered over a gap; the cabinet was a featureless slab (doors, louvres and handles on both long faces, a lighter paint); the skip's load was a flat paved lid (the walls now stand above a heaped load); the road barrier read as planks (thicker beams and posts); the scooter was the ground's khaki (now teal) and stood on a stand with both wheels down; the cabin's door faced away from every close view.
- Left: the car wreck reads as a rusty slab or tray rather than a car. It is a consequence of the 0.7 m remains height ([choices](../choices.md)). The planter's shrub is smooth lumps; wires and tubes (fence, scaffold) and the cone vanish by the mid battle distance, as their size makes them.
- Not ours: the workbench's dark axis bands and its soft shadows; the tree's crown and its ring.

A second critique was not run on the changed sheets; they were looked at.

**Not done.** No battle or scene was run: nothing places these props yet (C46). The wasm was not rebuilt; the appearance bindings in the fixture changed (status removed, three `modular`), so it must be rebuilt before a map places a street prop. A traffic cone blocks no mover, so the renderer treats it as a walked-on surface and ground paint lands on it.

**Pictures** (scratch): `throwaway/c45/final-group-a.png`, `final-group-b.png`, `final-group-c.png` (one sheet per kind, eight views each with the 1.8 m figure and the body's box); the first round is `group-a.png` to `group-c.png`; single sheets in `throwaway/c45/sheets2/`.

## Street trees (2026-10-02)

**Contract as landed.** A street tree is drawn as a tree. The `street_tree` row's binding is `drawn_by: "forest"` (the trunk row's, which it extends), no longer `systems_only`, and the scenery draws every tree body: a forest's trunks by the forest's rule, and any trunk no forest generated as a lone tree (`placeForests` in `scenery/placement.ts`). A lone tree is one of the species `trees.lone` names in the biome (`tree_tall`, `tree_birch`), 0.85 to 1 of its own body's height (8.5 to 10 m on the generator's 10 m box) and 0.7 to 0.85 of its appearance's width (a crown 5 to 6 m across), seeded by its body. It casts and takes shadow, is fogged and is gone where its ground is seen cleared, as a forest's tree. No art was built, no scenery kind added and no body value changed; `asset check` passes; the stand-in tint is deleted.

**Why this and not a kind of its own.** The street tree's box is the forest rule's trunk (0.35 m by 10 m), so the one-size tree art fits it as it fits a forest's ([choices](../choices.md)).

**Checks.** `web/tests/scenery.test.ts`: a tree body outside every forest is a tree of its own height, of a lone species; on the generated map every tree body is one drawn tree. `web/tests/propAppearance.test.ts`: no tree is a stand-in. The `generated` scene: trees drawn are the trunks plus the street trees (44,061 = 43,534 + 527), and the structures no longer count them.

**Critique** (unprimed, `street-tree-65`, a new rig station on the avenue nearest the town's centre, shot with the street's models). They "read as trees", 8 to 10 m, "level with the eaves of the three-storey block", consistent with the cars and a bench. What is wrong is where they stand, which is the map's: "beside buildings, not beside a street", on the lawn between walk and facade about a metre from the wall, so crowns pass through walls, windows and roofs, and one rises from behind a roof. Also: trunks "stumpy" (0.6 m across, the body's own width, with 2 m of clear stem); one trunk leans enough to look toppled (the species' art); cream birch boles against yellow render.

**Open.** Placement (the street-placement slice's): a tree needs about 3 m to the nearest wall for this crown, or the biome's `lone.girth` comes down. A lone crown stands where the simulation has no foliage ([choices](../choices.md)). A map's own `trunk` prop outside its forests is now drawn too.

**Pictures** (scratch): `throwaway/final/generated-street-tree-65.png` against `throwaway/before/generated-street-tree-65.png`.
