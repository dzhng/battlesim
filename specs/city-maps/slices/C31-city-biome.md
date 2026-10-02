# C31: urban and plain ground composition

**Depends on:** C28, C52, C83. **Kind:** slice.

## Question
Does one generated map read as town and open country without a city-wide biome suppressing its plains?

## Contract it unlocks
One spatial composition from `MapDefinition.surfaces` and `MapDefinition.land_regions` (C04): pavement and urban yards suppress farm plots; the reserved plain retains fields, meadow, grass and optional forest from the ground lane. Grass grows only on unpaved surfaces. The grass build's per-clump loop over props is indexed (`grassPass.ts:243, 685-693`). The urban/plain boundary is visible in the full frame but is never a second sim rule or a renderer-only guess about roads.

## API seam
`packages/battle-renderer/src/terrain/{biome.ts,plots.ts}`, `grassPass.ts`, `fixtures/biomes/`.

## What the human can run or see
A paired town-edge and open-plain crop from the same generated seed.

## Verification
- Grass build ms at generated town prop counts; open fields remain in the same map.
- Village grass unchanged.
- Frame-cost row.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the paired crops (**urban suppression and retained plain fields only**) against **C28's pavement shot and the accepted ground-lane field shots**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Pavement, curbs, markings.


## Delegated to the implementer
Urban yard and park appearance; grass density. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Village biome.

## Feedback that would change this slice
Urban/plain transitions that confuse movement or scale reopen composition, with accepted ground variables held fixed.

## Outcome

**Built.** A generated town's ground is its yards, with meadow commons round them; the country road through it is a street as far as the yards go; the plain beyond is the fields it was. Nothing in `crates/` changed.

**What the renderer can know.** `MapDefinition` has no `land_regions`: the compiler refuses a plan that names one. The generator's settlement outlines and blocks leave it only as encounter sites, for the planner; no map holds them, saved or generated, and they reach neither the page nor the renderer. What every map does hold is its buildings, so the urban ground is the plot rule's, which already read them ([`choices.md`](../choices.md)).

**The seam.**
- **`field_rules`** (`terrain/plots.ts`): a plot whose centre lies within `yard_m` (40 m) of a building is `settlement_kind` (`yard`); any other within `settlement_m` (110 m) is `surround_kind` (`meadow`), where no crop is drilled. Neither draws a kind, so every plot beyond is what it was. The biome refuses a drilled crop for either.
- **`roads.<kind>.town`** (`{ kind, beside_m, gap_m }`): where a yard lies within `beside_m` of a road's edge on both sides, the road is drawn by `kind`'s row, on across gaps shorter than `gap_m`. The summer country road's is `road`: asphalt, walk, curb and lines.
- **`TerrainSurface.strokes`** (`drawnStrokes`, `terrain/surfaces.ts`): the exported strokes as drawn. A road stops, round, where a street's run starts; the run is laid on the same line and as wide, cut square the street's `join_m` (3 m) past each end, under the road. No edge moves, and the simulation's distance is the same. `terrainField(surface)` builds the surface field from them; the material and the rig both use it.
- **A carried join starts in drifts**: where one road lies under another, the line the upper surface starts from wanders by the carry. Gravel spills onto a street's end; a track's earth onto the road it joins.
- **`groundMarks`** lays a crossing beside a crossing stretch's own length only.
- **The grass build's prop loop was already indexed** (`packGrassProps`: footprints bucketed by grid cell, a clump asks its own cell's). Not rebuilt.
- **The rig**: stations `town-edge-250`, `town-edge-65`, `road-join-65`, `road-join-25` stand on the renderer's own ground, rebuilt in the page from its generation request; `TOWN_ONLY=1` runs the checks below.

**Measured** (mixed, medium, seed 2). The comparisons with the starting commit were taken before the river recut that followed in the same pass, which reshuffles the fields of every map with a river; the rest are the final state's.
- **Plots**: against the starting commit, 3,093 of 18,861 changed kind (the old settlement green: 2,531 to yard, 562 to meadow) and no other plot changed kind or colour; on the village 12 of 4,122.
- **The plain**: `country-250`'s frame, bare ground and class mask were byte-identical to the starting commit's. By the class mask 51% of its open ground is drilled (70% before the recut), of six crops, and none is yard.
- **The town**: at `town-250` every open-ground pixel is yard and none drilled. Against the generator's own blocks, 85% of block ground is yard and 0.3% a drilled crop. `town-edge-250` holds 26% yard and 20% drilled fields in one frame.
- **On screen** the town's open ground went from chroma 35 to 24 at the same lightness (L\* 37 to 36); the plain's fields are 22.
- **The road**: of 946 points sampled along the country road inside a settlement and beside one of the generator's blocks, 903 are street (the rest run between a hamlet's wide lots); of 2,746 more than 200 m outside any settlement, none. At `road-join-65` the street's roadbed reads 0.159 against the gravel's 0.250 (0.64 of it), and greyer (0.10 against 0.23).
- **Grass**: at `road-join-65`, of 33,296 clumps none stands on either road or on the street's walk; 5,383 stand in the 5 m past the walk and 721 on the country road's shoulder.
- **The village did not move**: `bend-65`'s bare ground and class mask are byte-identical; its clumps at four stations (35,372, 49,879, 36,176 and 34,952) are the same rows, root, height, kind and colour.
- **`fog-look`** passes under every style at every framing. `grey-veil` at `default-wall`: darkest seen 45, darkest unseen 39 (the starting commit: 44 and 39).
- **Build cost** on that map, once a world: the plot split 41 to 46 ms, the drawn strokes 3.5 ms, the field's index 10,450 to 10,453 KiB. The grass's prop table: 134,529 footprints in 25 ms, 7.9 MB.
- **Frame cost**, the roads' wear on against off, four interleaved pairs of 120 frames (before the street's carry under the gravel, which adds no term): village `bend-65` −0.03 ms (+0.60, −0.09, −0.03, −0.12), `town-65` +0.12 (+0.12, +0.12, +0.13, +0.05), `junction-65` +0.04 (+0.04, +0.02, +0.07, −0.01), `road-join-65` +0.06 (+0.07, +0.06, −0.20, 0.00), on a 2.2 to 2.7 ms frame. The yard is data: it has no term of its own.

**Compared** (before against after at identical framing; `broken-arrow/town-districts.jpg` for the read). The town's ground left the lawn green for the fields' olive family, as the reference's does; the reference's courts and car parks between blocks have no counterpart here.

**Critique** (unprimed, two rounds; the second on the final shots). Both: no crop rows or tractor lines between the town's buildings; the town's end is told by its streets and roofs; no halo. Round one, on a mown lawn with the street cut square where the yards end: "a ruled line with an airbrush smudge" at the join, walks running on beside a wheat field, ochre "stains" on the lawn, a fan of crossing bars. Changed: yards on both sides for a street, the street carried under the gravel, the yard's patches softened, the crossing rule. What round two still says:
- **The join still reads as a glitch** (high): gravel as "a soft lobed blob" over the asphalt on one side and a ruled cut on the other, a dark patch inside it, a crossing's bars half covered. On this map it falls at the corner of the town's last street, where the country road meets it off its middle.
- **The ground between buildings is one unbroken stretch of rough grass** (high): no yards, paths or paving; shopfronts open onto it; block interiors read as vacant lots. The yard's green still runs one parcel past the last street, and from 2,500 m the town is "a slab on more saturated green".
- **A building's shadow along a street reads as a second asphalt** (high), with slits of sun between buildings as streaks. It was the gravel's finding (C28); the surface is now asphalt and the shadow the same.
- Parcel lines (the verge) run between a suburb's houses; the desaturated fields beside the yards can pass for a cloud's shadow at 65 m (medium).

**Open.**
- **The join** wants its own term along the road (a broken edge across the whole width, the walk ending built), not the carried blend two crossing roads use.
- **An apron round each building** (paving or worn earth, grass kept off it) is what would make the ground urban. The footprints are in the terrain's site; the surface field does not list them, and listing nine thousand more records in a town is a cost to measure first.
- **The yard's colour and grass are held by `fog-look`**, not chosen: lighter, smoother or greyer fails `grey-veil` at `default-wall` (SG4's margin, measured again here).
- **Where a town is** should be the map's to say. Until it is, a hamlet's road between wide lots stays gravel beside asphalt side streets, and a yard's edge is a plot's.
- **The river lab's fields moved** with the river recut: two road checks of the `ground` scene now fail at its `track-65` (the track's earth, 0.211, beside a rapeseed field's 0.225). They held by which field lay there.
- No checkpoint was shown in Preview (the brief's): the shots are in the report.
