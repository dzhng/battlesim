# C27: ruin gutted art

**Depends on:** C14, C42, C43. **Kind:** slice.

## Question
Do ruin and gutted buildings read as the same building, switching when the side learns of it?

## Contract it unlocks
C14's placements are drawn on the published collapse or gutting, via the knowledge path (`KnownProp.replaces`). The far tier has matching ruin and gutted variants.

## API seam
`propAppearance.ts` state switch, C22's chunks.

## What the human can run or see
`/city` frames of a collapse and a gutting, plus the replay picture.

## Verification
- No visual collapse of a still-standing collider.
- Knowledge isolation: the other side sees the old state until it learns.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**terminal silhouette and burnt read only**) against **C22's intact shot of the same buildings**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Street props, fog.


## Delegated to the implementer
Burnt treatment. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Sim owns timing and dimensions.

## Feedback that would change this slice
Terminal appearance that hides a standing gutted body or suggests intact cover reopens terminal art/fit.

## Outcome

**The contract as it ended up.** A building is drawn intact, as its template's `ruin` rows or as its `gutted` rows, by what the observed side was published about it, and by nothing else.

- **What the simulation publishes** did not change. Destroying a building replaces every part with a prop on the same plan: a `ruin` at the building's one ruin height where it collapses, a `gutted` shell at the part's full height where it is taller than six floors. A side that sees any part's footprint learns all of them, as known props that name the part they replace (`authoredProp`).
- **The state** is `fallenBuildings(index, known, shells)` (`models/buildingReferences.ts`): a building with a part known as one of the catalog's gutted shell types (`buildingRemains` in scene-assets reads them off the building prop types' `destroyed` rows) is `gutted`; with a part known replaced by anything else, or destroyed, a `ruin`; with nothing known, intact. A remains' height never decides. A `FallenBuilding` is `{ building, state }`.
- **The drawing** (`models/buildingPlacements.ts`): a destroyed building leaves the intact rows and is drawn from the fallen population as its template's rows for that state, at every tier those rows have, and the coarsest of them cast for it. A template with no rows for the state is refused by name (`state.missing`).
- **The same knowledge** feeds the camera's obstacles (`buildingObstacles`), the fog's occluders (`knownOccluders`) and the smoke over a destroyed building (`effectFeed.ts`, looks `gutted` and `ruin` in `presentation.effects.smoke`).

**Deleted.** The box fallback for a fallen part with no art, `BuildingScene.box`, `buildingKits` (a map asks for `templateKits`), the prototype kit every map with buildings fetched for it, `presentation.buildings.ruin_tint`, the camera lab's `seenFallen` and its `fallen.remains_height_m`, and the city block lab's invented 1.5 m remains. Stand-in boxes for props with no art still use the prototype kit, on their own.

**Where it is seen.** `/lab/city-ruins` plays the saved encounter `camera-lab/shelling`: HE bursts over the yard of the five-floor U block and in front of the twenty-floor tower, a blue squad that watches both, a red squad behind the two slabs that walks out at tick 600. A switch draws either side's view. `/lab/city-lineup?state=ruin|gutted` now holds only the templates that end in that state.

**Proved.**

- `web/tests/buildingKnowledge.test.ts`, in the built WebAssembly: blue is published the block's three parts as `ruin` at 4.075 m (a quarter of 16.3 m) and the tower's as `gutted` at 61 m, red nothing; the fog's occluders are those heights for blue and the map's for red; red still knows both intact when its walk starts and knows what blue knows after it; each known part is a smoke source of its look. A squad placed where one wing of the U block is out of its sight is published all three parts.
- `web/tests/buildingPlacements.test.ts`: the classification (remains, a shell, nothing seen, a wreck, one part of a compound, parts known both ways, a remains as tall as its part), a collapse and a gutting drawn as their own rows at each tier with their own casters, a missing state refused with the scene unchanged. Four mutations each turn it red.
- The `city-ruins` scene, at five fixed cameras (each building near and from 1,384 m, and the whole map at 2,000 m), units, grass and effects off:

| Check | Measured |
|---|---|
| Blue's block is inside the remains it knows | 0 stray pixels at every station; intact, and for red, 31,906, 200 and 81 pixels stand above them |
| Blue's tower stands through its full part | every core sample covered; inside its part near and far (the whole-map view has the map's edge behind its roof) |
| Blue's picture changes where each stands | 64% to 78% of the pixels over each building |
| Red's picture is its picture from before | 0 pixels differ at every station |
| Switching sides switches the picture | 78% to 94% differ; blue again, 0 |
| The gutted tower is darker than intact | brightness 118 to 78, 125 to 84, 123 to 83 |
| The far tier | past the last boundary and at the whole-map view blue's destroyed buildings draw 2 instances, both at tier 3; near, 50 at tier 0 and 13 at tier 1; red none |
| Camera and fog | blue's eye passes 6 m over the remains and is pushed back from the shell; red's is lifted over the block; occluders 4.075 m and 61 m for blue, 16.3 m and 61 m for red |
| Smoke | 4 sources for blue, none for red |
| Red, later | still intact at tick 600, 88 ticks after blue knew both destroyed; knows both by tick 1,080 and draws them as blue does |

- `city-lineup` (apartment set): 7 templates intact, 5 in `ruin`, 2 in `gutted`, each drawn and inside its parts. `camera`, `garrison` and `city-block` pass.

**Frames** (`throwaway/evidence/city-ruins/` after `scene -- city-ruins`): `<before|after|learned>-<blue|red>-<block-near|block-far|tower-near|tower-far|overview>.png`, `crop-<block|tower>-<phase>-<side>.png`, `classes-*.png` (the ground-classes view the fit is judged in) and `battle-after-blue*.png` (the battle as played, with smoke).

**Against the intact shot** (compare-screenshots, blue before against blue after, same camera): the block's crop keeps its edge energy (ratio 1.11) and brightness (84 to 86) and differs in 27% of its pixels; the tower's crop is darker (107 to 83) with fewer edges (0.87). At the far stations the two frames differ in under 0.1% of the picture. Verdict: the destroyed frames are the intended difference, and at range it is small.

**What an unprimed critic saw**, and where each finding went:

Two passes, each a fresh agent given every frame and no history. The first saw the frames with effects off and bursts walked in a line; the second the final frames, with the bursts scattered and smoke in the battle frames. Both found, without being told, that blue's redrawn frames are identical, that red's frames do not change between before and after, and that red's later frames show what blue's show.

| Finding | Disposition |
|---|---|
| **The collapsed block is unmistakably the same building, destroyed, up close** (its plan, its wall stubs with their openings, a patch of its roof on the heap). From 1.4 km it is a pale U a few pixels high with no shadow, and at the whole map a smudge that could be a bare lot | Kept for the art: the remains are the simulation's box, and the coarse ruin is one row (C14, "not done, and known"). The dust over a fresh ruin helps at mid range |
| **The gutted tower reads as gutted only to a careful eye up close, and as a darker or shaded building from far off**: its silhouette, roof line and shadow are the intact tower's, with no soot streaks, broken edge or debris; the roof darkens evenly while some roof furniture goes pale | **Reopens the terminal art (C14)**, as this slice's feedback rule says: a shell that could pass for intact cover. Not changed here: the kits are being edited by another pass. The smoke added in this slice is what made the second critic read it as burning at mid range; how smoke reads from the far stations is not captured |
| The first pass's "brown ribbon" at each building's foot | Fixed: the bursts are scattered over the yard and the ground in front of the tower, not walked along a line |
| The crater field is a saturated, high-contrast pattern clipped to where the bursts fell; the ground outside the ruin is untouched; the heap is a smooth sheet with a pale rim and loose cubes | The scars' look is the ground lane's. The clean edge and the heap are the art's (C14's open list) |
| The fog wedge behind the block is the same after it collapses | Correct by the rule: 4 m of remains still hide flat ground from a 1.7 m eye, and the occluder is the remains' box. It reads as wrong because the wedge is the strongest mark of a building that is now low. For the fog look (C20) |
| The fog's white lines end loosely on the heap's flanks; in red's later view the fog crosses the yard while the heap on both sides is drawn clear; a building half in fog shows a colour seam; shadows under fog are a darker fog; pale fields under fog look like holes | The fog look's (C20): an occluder takes fog whole. Recorded in C22 before this slice |
| Dark field parcels beside the tower read as a second shadow | The ground lane's |
| The dust is the rubble's colour and hides its detail; the tower's smoke is brown-grey against the dark fog behind it and has no visible source | Open: the look numbers were set from one battle. Smoke from the openings needs the art to say where they are |
| Far facades alias to stripes; units are specks | C23's record; not this slice |
| Nothing in red's picture marks its intact buildings as possibly out of date | By the rule: a side draws what it knows |

**Answer to the slice's question.** A ruin reads as the same building, destroyed, and switches exactly when the side learns of it. A gutted tower switches at the same moment and stands as the simulation says, but its art does not yet say "burnt out" without the smoke.

**Not verified.** No Preview checkpoint was opened (several agents were running). Smoke from the far stations. The frame cost of the smoke over many destroyed buildings: sources share the one smoke budget, and nothing was measured.

Decisions: [choices](../choices.md#c27-ruin-and-gutted-art-drawn-by-what-a-side-knows).
