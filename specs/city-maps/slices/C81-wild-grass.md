# C81: wild grass

**Depends on:** C80. **Kind:** slice.

## Question
Do wild-ground species (short meadow, tall rough, dry prairie, weedy verge) read as distinct at the default camera?

## Contract it unlocks
Biome growth rows point meadow, pasture, verge and rough ground at C80's presets. The grass kind and growth-row caps (16 and 16) hold.

## API seam
The biome's `grass.growth`, `grassPass.ts`.

## What the human can run or see
`field-65` and `bend-25`.

## Verification
- Grass ms and clump counts; a frame-cost row.
- **Kill if** it's >+0.4 ms over today's 1–1.2 ms.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**a single-species meadow plot**) against **`../assets/reference/ground/grass-a.jpg`, `manor-ground-closeup.jpg` and C62's baseline**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Crops, palette, within-field variation.


## Delegated to the implementer
Preset parameters. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Grass exclusions.

## Feedback that would change this slice
Grass silhouette that appears taller than accepted cover changes species/placement height within the shared cap.


## Outcome

**Built.** Four wild kinds from the generator (`grass_meadow` short meadow, `grass_rough` tall rough, `grass_prairie` dry prairie, `grass_verge` weedy verge) and two new plot kinds, `rough` and `prairie`, beside meadow and pasture. The biome's growth rows point meadow, rough, prairie and the verge at them (as mixes since C82). Nine growth rows and ten kinds: inside the caps of 16.

How a clump is shaded became the biome's, because the old look failed at the play camera: `soften_m_per_px` and `soften` (a clump's dark roots, pale tips and blade facing give way to the ground's as it shrinks on screen), `blade_facing`, `min_blade_px`.

**The rig** gained what the slices needed (`web/scenes/_groundStations.mjs`): a station over the village's roomiest open plot of each kind (`meadow-65`, `rough-25`, …, following the patchwork when kinds or weights change), `stationPose` and `stationReport`, and on the grass pass a `retune` probe that regrows by other rules in the same page. `web/scenes/_grass.mjs` (in the `ground` scene, `GRASS_ONLY=1` alone) checks that each kind of ground grows what its row names and that no clump stands on the road or under the wood, by the class mask.

**Frame cost** (paired grass on and off, 240 forced frames a batch, five pairs, median; Apple M-series, the machine shared and loaded):

| Station | Clumps before → after | Still camera, ms | Moving camera (regrows every frame), ms |
|---|---|---|---|
| `field-65` | 26,421 → 37,248 | 1.92 → 1.16 | 1.69 → 1.45 |
| `bend-25` | 22,167 → 22,705 | 0.84 → 0.91 | 1.22 → 0.85 |

Inside the +0.4 ms kill bar at both, with a third more clumps and blades held 1.4 px wide where they were 0.8. The baseline's pairs spread from -0.1 to 3.3 ms, so the bar is met, not beaten by a known margin.

**A regression found and removed on the way.** C82 grew the field's uniform struct to 2.3 KiB, and every shader stage took it with `let P = params`, a whole copy per invocation: the vertex stage, per blade vertex, cost 8 to 12 ms a frame over a field (measured at `meadow-65` and `wheat-65`). They read it through a pointer now. Nothing else in the lane should grow a uniform that a vertex or fragment stage copies.

**Compared** with `grass-a.jpg`, `manor-ground-closeup.jpg` and the starting commit's stations: less wrong at 25 m (a closed sward of wide blades; drifts of taller, drier grass and weeds), and at 65 m no longer dark pepper flecks over flat colour.

**Critique** (two unprimed passes; the second after the changes the first caused). Distinct at 25 m: prairie (thin dry tufts over khaki). Not distinct on either pass: meadow, rough, hay and pasture, which differ in height and mix but sit on near-identical palettes; "rough ground is not rough". At the play camera both passes found the ground cover reads as flat colour with speckle, with no blade, height or density to see. So the slice's question is answered no: at the default camera the wild kinds do not read as distinct by shape, only by what their plot's colour and grain say.

**Open.**
- At 65 m a blade is under a pixel. What separates kinds there is the plot's palette (rough and prairie borrow meadow's and hay's until C84) and the ground's own texture between and under the clumps (C85). Grass alone will not make the play camera read as vegetation.
- No grass is drawn at 250 m (the fade ends near 180 m); `field-250` and `country-250` moved only because plot kinds did.
- The village scene's `GRASS_COST` run threw on a missing `median`; it is defined now, and not rerun under the GPU budget.
