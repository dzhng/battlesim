# C28: pavement

**Depends on:** C03, C63. **Kind:** slice.

## Question
Do roadbed and sidewalks cover exactly the right ground and never read as fog or shadow?

## Contract it unlocks
City pavement reads **C63's surface distance field** (the one owner of distance to roads, forests and water; it already replaced the per-fragment loops). This slice adds the city surface kinds (roadbed, sidewalk) and their palette; it bakes nothing of its own.

## API seam
`packages/battle-renderer/src/terrain/`.

## What the human can run or see
A top-down crop of a block, and the village terrain before and after.

## Verification
- Surface and export agreement.
- Texture ≤16 MB.
- Terrain GPU ms.
- Village terrain compared (must be unchanged within tolerance).

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**pavement coverage only: "does any road read as fog or shadow?"**) against **an aerial reference crop of the real block (never shipped), and the village's pre-change frame**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Curbs (C29), markings (C30), urban/plain composition (C31), props.


## Delegated to the implementer
Palette; texture resolution. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Terrain height; speed classification.

## Feedback that would change this slice
Pavement that reads as a separate painted overlay reopens road-ground composition within the shared surface field.

## Outcome

**Built.** A town street is asphalt with a paved walk each side, and a town's yards are paving. Nothing is baked: the look reads C63's field, as the country roads do.

**What the maps hold.** A generated town's streets are `road` strokes (7 m, avenues 10 m) and its loading yards are `road` polygons. No map holds a `sidewalk`, and nothing the export carries says where a town is. So the slice's "sidewalks cover exactly the right ground" has no ground to check against: the walk is a look, as the shoulder is.

**The seam.**
- **A road row may say more** (`biome.roads.<kind>`, all optional): `area`, the row that draws the kind's polygons; `walk`, a band beside its strokes and the row that draws it; `layer`, its place where two kinds overlap. The summer biome has rows for `road` (asphalt; its yards and a 2 m walk drawn by `sidewalk`; layer 2.5, under a country road) and `sidewalk` (slabs, a joint every 2 m).
- **`groundPaved` returns two distances.** `drawn`: how far inside each row's paving the point lies (a stroke by its kind's row, a polygon by its kind's `area` row, a walk by `walk.kind`'s). `rule`: how far inside the simulation's paving. The colour and the grass read `drawn` (no grass grows on a walk); the class mask reads `rule`, so it still says only what the export says.
- **It also returns the stroke's run**: its direction, and how far along the stroke the point is. The field's paved records now hold each stretch's start along its stroke (`SURFACE_STROKE_ALONG`); a walk's joints, and C30's dashes, are laid out by it.
- **`join_m` is the lower road's own carry**, and only a carriageway is carried: a track's earth still runs 2 m onto the road it joins, a street ends on the country road's edge.
- **A map that names no kind but `road`** (the village, the geometry lab) has its roads drawn by the country road's row (`drawnKind`, [`choices.md`](../choices.md)).
- **The street-to-sidewalk edge** the roads pass asked for is routed round, not solved: a street is a stroke, so its edge is the stroke's own. Two kinds of polygon side by side still share one distance.

**Measured** (the `ground` scene's `STREETS_ONLY` checks at the generated town's `junction-65`; displayed luminance in the sun, each surface's upper quartile):

| Roadbed | Country road's gravel | Walk | Lawn beside the walk |
|---|---|---|---|
| 0.156 | 0.246 | 0.334 | 0.091 |

- The roadbed is 0.64 of the gravel and greyer (red less blue over red: 0.11 against 0.24), and 1.7 times the lawn. The aerial reference's asphalt is 1.1 to 1.3 times its grass and its concrete yards twice.
- The walk's pixels are the bare ground's with the grass drawn (a mean difference of 0.04 of 765, against 3.3 on the lawn).
- **The village did not move:** `bend-65`'s bare ground and class mask are byte-identical to the starting commit's. With grass the frame differs by 7,975 pixels, and the same build shot twice by 8,129 (grass depth ties). The river lab's `junction-65` and `track-65` bare ground is byte-identical too.
- Every generated station's frame differs from before: 1.6% (at 2,500 m) to 12% (at 25 m) of its pixels by more than 32 grey levels. With the pavement alone the class masks were byte-identical to before at every station from 25 m to 250 m; C30's wider reach then moved them.
- No texture and no buffer: a row of the look table grew from 8 to 14 vec4 (with C29's and C30's), and the field's paved records use a float they already had.
- **Frame cost**, the roads' whole wear on against off (streets' grain, joints, curb and lines with the country roads' wear), four interleaved pairs of 120 frames. At load 30 to 70 nothing resolved (pairs of ±3 ms). At load 9 to 25, on the final shader: +0.06 ms at village `bend-65` (+0.80, −0.22, −0.15, +0.06), +0.08 ms at `town-65` (+0.38, −0.54, +0.08, −0.02), +0.18 ms at `junction-65` (+0.06, +0.18, +0.26, −0.14), on a 2.3 to 2.6 ms frame. The rows, the walk and the paint order have no switch and are not in that number: they add one `max` a stroke to a loop the field already walks.

**Critique** (unprimed, two rounds, on the pavement without curbs or lines). Both named the surfaces correctly ("asphalt street", "concrete slab sidewalk") and found no road that reads as fog. Round one: a street's end blurred into the gravel beside sidewalks cut on a ruled line; gravel and sidewalk close in colour. Changed: `join_m`, cooler asphalt and paving. What round two still says:
- **Shadowed gravel is the tone of sunlit asphalt** (high confidence, both rounds): a building's shadow across the country road reads as a change of surface, and the sunlit slivers between two shadows as pieces of sidewalk. It is the gravel road through a town of three-storey shadows; no asphalt grey above the lawn's luminance avoids it.
- The country road does not read as gravel ("a sandy lane"), looks like the widest road in town, and cannot be found at 2,500 m; its verge is "watercolour blotches". Those are the country road's look and the town's composition (C31).
- A street ends on the gravel in a square cut, and so do its walks.
- Walk corners are sharp mitres: a walk is its stroke grown by a width.
- Asphalt, slabs and gravel share one grain at different tints.

**Open.**
- The country road through a town should be a paved road there: the map lane's kinds, or C31's composition. Most of what the critique still finds follows from it.
- The village's map should say `country_road`; then `drawnKind` goes.
- A map's own `sidewalk` areas take the sidewalk's row but no map has one; no station stands on a yard.
- No checkpoint was shown in Preview (the brief's): the shots are in the report.
