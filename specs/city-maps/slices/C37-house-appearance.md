# C37: shared house appearance

**Depends on:** C01, C13, C32/C14 and C22. **Kind:** slice.

## Question
Can all existing house instances use the template appearance owner with their original physical shape and look?

## Contract it unlocks
Use C01's original house descriptors and the existing house source to produce reusable template placements. Cut over every current `drawn_by: building` consumer, including village and labs, through C32/C22. Delete that appearance branch, its per-house bundle plumbing and any unreferenced bundles in the same pass. Keep source meshes still shared by the template/module owner. No surroundings or house dimensions change here.

This is the removal condition for C22's temporary coexistence with the house path. C34–C36 subsequently add surroundings; they cannot inherit an appearance branch whose remaining consumers were forgotten.

## API seam
Existing house appearance source and C01 physical descriptor → C13/C32 reusable placements → C22 chunks. One property/loader/render cutover across existing consumers; no compatibility alias.

## What the human can run or see
Matched village/lab house crops and a consumer inventory showing the old appearance path is unused and deleted.

## Verification
- Preserve original physical geometry, PropIds and terminal/side-knowledge behavior; name any presentation-schema/config identity change separately from physical outcomes.
- Every existing house consumer resolves supported intact/terminal templates before deleting the shared branch.
- Matched frames preserve the accepted house look and remain within frame/startup budgets.
- Judge house appearance parity only; town layout and surroundings are later slices. Use compare-screenshots against current house crops, run unprimed screenshot-critique last and preview-shots non-blocking.

## Delegated to the implementer
Source/module grouping within the accepted house fit and C32 codec. Geometry changes and retaining duplicate appearance owners are not delegated.

## Must stay green
Original physical outcomes, house visuals, learned damage timing and one template appearance path.

## Feedback that would change this slice
An appearance/fit regression reopens its reusable source mapping before the surroundings cutover.

## Outcome

**2026-10-02: every building on every map is drawn from the template art library.** The fitted house path is deleted: no `unit: "building"` appearance, no `house_a/b/c`, no building half of `PropAppearances`. The decisions are in [choices](../choices.md#buildings-lane).

- **The contract that changed.** A city set names the physical catalogue it dresses (`city_sets.<set>.catalogue` in `assets/catalog.json`: `generated`, the map generator's, or `authored`, the hand-authored maps' boxes). One library covers both: `covers` lists both hashes (library format 2), each catalogue is held to its own sets, the generator's to `require_complete` and the authored to `validate`. Art identity is still in no map.
- **The set.** `village` (`blender/city/village.py`) dresses all twelve boxes of `fixtures/building-templates.json`, each with one module standing and one fallen, built by `house.py`'s farm at the box's own size. Every one of the twelve is placed by a saved map. The three village houses are the same triangles as before at every tier. A lab's box borrows the nearest house's look a tier coarser; the kit bakes to 37 MiB against a 50 MiB budget.
- **Physics.** Untouched: the village's digest and replay tests pass unchanged, and no map, catalogue hash or prop id moved. The one schema change outside art is presentation only: the prop row's `remains_state` is gone (fixture, resolved catalog, `contract::catalog::PropAppearance`).

**Who drew a building the old way, and what draws it now.**

| Consumer | Before | Now |
|---|---|---|
| The battle (village, endurance, projectiles, benchmark) and the labs that fed `buildings` (garrison, weapons, movement, fog, authority, contacts, consequences, river) | `structureModels` fitted `house_a/b/c` to each box; a known ruin was the house's `ruin` state | The building layer, from `session.buildingsFeed`: the template's `intact` rows, or its `ruin` rows once the side has seen it fall |
| Labs that took houses from the static world (ambush, sensors, readouts; geometry and ballistics, which have no battle) | `buildWorldLayers` fitted them as world props | The same feed (`useStandingBuildings` where there is no side) |
| The models layer's install (`mapAppearances`) | The map's house bundles | The kits the map's own templates draw from (`buildingKits`) |
| The workbench and `asset sheet` | `house_a/b/c` as building appearances with a box overlay | `city_kit_village`, a kit like any other; the unit selector has no "building" |
| The asset check | `fit.footprint` on a building's states, at the one ruin height | `templates.fit` and coverage, per catalogue |
| Rubble and a ruin no building owns | The scenery appearance `village_ruin` | Unchanged (its source renamed `ruin.glb`) |

**Pictures.** `throwaway/c37/before/` and `after/` hold every authored map's buildings at four fixed cameras with grass, effects, fog and cast lights off; `diff/` the comparison; `before-evidence/` and `after-evidence/` the garrison and village scenes' frames.

- **The village's three houses:** 7 of 12 frames are pixel-identical and the rest differ in under 0.2% of pixels, in a neighbouring house far behind the subject, which the building layer draws at another tier than the models layer chose.
- **The labs' nine boxes** differ in 2% to 17% of pixels. The farm is no longer stretched: wings keep their 7.6 m depth, so a long box (the ambush lab's 30 × 80 m) has a long yard where it had fat wings, and a tall one a steeper roof. Up close a lab's house is plainer than before: no tile courses on the roof, no glazing bars. At the tactical camera the two read alike.
- **A fallen house** (the garrison lab) draws its `ruin` rows: the same shell of broken walls, rubble and beams on its plan, with coarser heaps.
- **GPU memory** in the village: buffers 183 to 215 MiB (the kit is 15 MiB more than the three houses were, and the buildings' pool is 8 MiB); textures unchanged.

**Checked.** New and rewritten tests: two catalogues in one library and their refusals, a map's kits, missing art refused by name, the shipped village's houses standing and fallen from the shipped library (`villageBuildings.test.ts`), each falsified by a mutation. Scenes on the real GPU, on this branch before main was merged in: `garrison`, `workbench`, `city-block`, `camera`, `geometry` and `generated` pass. `village` fails the two checks it failed before this change (the road's edge colours, and its smoke tour, which throws) and passes the rest of what it reached; its new house check failed once on a clause since removed (it counted tiers drawn while the camera looked elsewhere), with every other clause holding in that run. `weapons` fails one simulation check (a grenade's area target) that no drawing can move; it was not run on the base commit. After merging main: `garrison` and `city-block` pass, `asset sheet city_kit_village` renders, `workbench` fails its tree drop on main's new `fit.tree_size`, and `village` throws in its panel tour before it reaches the houses.

**Not done.** The damage pass's fit rule for a `ruin` state was not on main: the ruins are held to the intact parts. The byte cost of whole painted buildings is the reason a lab's box is coarser; a kit of shared modules would not have that cost, and nobody needs one for arenas.

**What an unprimed critic saw** in the after frames, and whose it is. Nothing floats or sinks, no roof is missing, no shadow is detached and nothing z-fights. What it did find:

- **Two levels of detail across maps:** the village's houses have tile courses and glazing bars; every lab's has a smooth roof and plain dark windows, blurred up close. This slice's, and deliberate (the coarser tier).
- **The weapons lab's shed reads squashed, and the ambush lab's long boxes as walled depots:** this slice's too. The first is the farm pressed to a 4 m box, as the fitted path drew it; the second is the farm unstretched on a 30 × 80 m plan.
- **Narrow boxes close the yard to a slot** (movement, geometry): the wings keep their depth where they used to shrink with the box.
- **Pale ladder patches at roof corners, the stable wing's roof leaning on the barn's gable with a lit tip, moiré on its shaded slope, blank outer walls, a ruin that reads as a tray with battlements:** the farm's own art, the same in the before frames. Not touched here.
