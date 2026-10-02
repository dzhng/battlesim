# C67: road shoulder

**Depends on:** C66, SG3. **Kind:** slice.

## Question
Does a road fade into the field through a worn shoulder, with grass thinning across it, never an outline or a shadow (Q-G2)?

## Contract it unlocks
- A shoulder of 2–4 m per kind, with a one-sided noise-jittered edge. Its luminance is at or above the grass's and it differs by hue.
- Grass density ramps across it from C63's field, replacing the flat `clear_m.road` margin.
- **The road-keyed branch of `groundVerge` (`terrainMaterial.ts:420`) is deleted**, so the shoulder is the one "beside the road" owner.
- Selo Empire's `RoadMeshBuilder` and `TerrainRoadWear` are technique only: rewritten from reading, nothing copied.

## API seam
`terrainMaterial.ts`, `grassPass.ts` density, biome road rows.

## What the human can run or see
`bend-65` with grass off, then on; `track-65`.

## Verification
- Per 0.5 m band, luminance is monotone or flat from core to grass and never dips below the grass band.
- Clump counts per 1 m band are monotone.
- The jitter is world-anchored.
- A frame-cost row.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**the road-to-field transition band (0 < SD < 6 m) only; critique: "does the road read as an outline?"**) against **`../assets/reference/ground/manor-path-aerial.jpg`, `manor-path-closeup.jpg`, `forest-road-summer.jpg` and C66's shot**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Ruts, grass species, field palette.


## Delegated to the implementer
Widths, noise scales, thinning curve. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
C66's core pixels; `fog-look`.

## Feedback that would change this slice
An abrupt or oversized shoulder changes shoulder falloff/width; road core colour and ruts are later variables.

## Outcome

**Built, as SG3's first fallback.** Beside every road the ground is washed toward a worn-earth hue and the grass thins across the same band. There is no band of a colour of its own: that was built first, and the critique read it as an outline (see [SG3](SG3-road-wear-read.md#outcome)).

**The seam.**
- **`groundShoulder(xy, footprint, paved)`** returns `(wear, kind)`: 1 at a road's edge, 0 past its shoulder. A shoulder is `roads.<kind>.shoulder.width_m` at the widest (3.5 m on the country road, 2.2 m on the dirt track); its outer edge wanders inward by noise fixed to the ground, never outward, and is broken toward it by tufts of the field. It fades out as a pixel grows from 3% to 8% of the width: whole at 65 m, gone by 250 m.
- **The ground** goes `shoulder.cover` (0.45) of the way to the shoulder's colour where the wear is whole. That colour is first scaled up to the luminance of the ground it lies on, so no shoulder can be a darker band, whatever the field.
- **The grass** reads the same wear: all of the field's clumps off the shoulder, `shoulder.grass` (15%) of them where the wear is whole, none where it is over 0.9. `grass.clear_m.road` is gone.
- **`groundVerge`** is the plot edges' alone; its road-keyed branch is deleted. The green line that ran along every road is gone with it.
- **`groundReach`'s paved reach** is the widest shoulder. The class mask's road distance is exact over that band.
- **`BattleFrame.setRoadWearShown`** (the lab's `suppressRoadWear`) draws the roads plain: paired frames, and `ROAD_COST=1` on the `ground` scene.

**Measured** (the `ground` scene's road checks; displayed luminance; village `bend-65`, bare ground):

| Metres from the road's edge | −1 | −0.5 | 0 | 0.5 | 1 | 1.5 | 2 | 2.5 | 3 to 5 | Grass, 5 to 9 |
|---|---|---|---|---|---|---|---|---|---|---|
| Luminance of the 0.5 m band | 0.243 | 0.242 | 0.161 | 0.137 | 0.132 | 0.127 | 0.122 | 0.118 | 0.117 | 0.123 |

- Every band is at most as bright as the one inside it, with grass on and off, on the country road and on the dirt track. No band is under the grass band by more than the fields' own spread from band to band (the open field 3 to 5 m out reads 5% under the field 5 to 9 m out, before and after).
- Clumps per metre-wide band beside the village's straight road, from its edge: 78, 194, 241, then the field's 233 to 258 (33% and 82% of the field in the first two metres). Before: 160, 255, then the field's.
- The wear is anchored to the world: after a 4.4 m camera move the same world points differ by 0.8 (of 765) and the same pixels by 57.
- The transition band (0 to 6 m out, `bend-65`): mean saturation 0.62 before, 0.57 after, against 0.43 to 0.49 in the three references; contrast across the band 0.21 before, 0.23 after, against 0.33 and more.
- The reach: the surface field's index went from 651 to 648 KiB on the village and 1,037 to 1,024 KiB on the dense synthetic town; the paved records a lookup visits at a play-camera pixel from 0.047 to 0.049 and from 0.49 to 0.56. The finest level was already built for a 2 m pixel.

**Frame cost.** The roads' wear (C66's surface detail and this shoulder) on against off in the same frozen frame, four interleaved pairs of 120 frames, load average 120: +0.10 ms at village `bend-65` (pairs −0.05, +0.10, −0.09, +0.27) and −0.02 ms at the generated town's `town-65` (+0.27, −0.02, −0.40, −0.09), on a 2.5 to 2.7 ms frame. That is under what four pairs resolve (about ±0.3 ms); one run, not repeated.

**Critique** (unprimed, two rounds). Round one, on a shoulder painted in a sand colour: "a sandy outline" in every gravel-road frame, a border "drawn round each lawn" in the town, "flame-like tongues" on its outer edge. Round two, on the wash: no outline at 25 m, and at 65 m only on one side of one frame; at 120 m and 250 m "a fuzzy halo fringe" that could read as a contact shadow. The fade by distance answers that; it was shot at 120 m and 250 m and looked at by its author only. No dark region on a road was read as shadow in either round. What it still says:
- the wash shows against dark fields and vanishes against stubble, so one road can have it on one side only;
- the tufts that break the edge are angular, "torn paper";
- crops beside the road still stop on a line parallel to it;
- at 250 m the road is "a flat strip laid over" the fields: field boundaries run into it and stop. Headlands and a verge that answers the road are the fields' (C82 to C85).

**Not run.** `fog-look` (must stay green): the shoulder is lifted to the luminance of the ground under it, so it cannot move that scene's darkest seen ground, and the GPU budget allows one scene a slice.

**Open.**
- The dirt track's bend in the river lab is a hard V and the village's road elbows at 250 m, in two critiques: the rounding takes at most a third of a run and half a width (C65). A gentler bend is a physical change.
- The shoulder stops dead at a bridge's deck.
