# C83: crops

**Depends on:** C82. **Kind:** slice.

## Question
Is farmland a patchwork of distinct crops, every one ≤0.9 m, with no false buff (Q-G10, Q-G11)?

## Contract it unlocks
Presets for wheat, barley, rapeseed, hay and stubble, pasture, and ploughed or fallow (no growth). Each plot kind picks its crop in the biome, within the 15 plot growth rows. No maize; visual only.

## API seam
The biome's `plots` and `grass`.

## What the human can run or see
`field-65` and a 25 m station per crop.

## Verification
- The C80 height validator passes.
- A grass ms row.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**each crop's plot, at uniform tint; critique: "does any crop look tall enough to hide a standing soldier?"**) against **battle-look's `brokenarrow/gameplay-trailer-08.jpg` and C82's shot**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Palette, ground texture.


## Delegated to the implementer
Crop weights. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
No sim effect.

## Feedback that would change this slice
Crops that imply protection they do not provide change height/coverage below the accepted effective-height cap.


## Outcome

**Built.** The patchwork's kinds are meadow, pasture, wheat, barley, rapeseed, hay, stubble, ploughed, rough and prairie (`fixtures/biomes/summer.json` `plots`), and each grows its own row: wheat, barley, rapeseed and hay are generated kinds of their own, stubble and pasture were re-cut, ploughed earth has no row and grows nothing. No maize. Ten plot kinds and nine growth rows, inside the 15.

A row's `rows` says how closely its clumps keep to the plot's drill rows, the ones the terrain already paints (`furrow_m`): a clump moves across them toward the nearest bright band, short of the plot's verge and any bare margin (`frame/grassPass.ts`). Wheat, barley, rapeseed and stubble use it.

**Visual only.** Nothing under `crates/` changed and nothing here is read by the simulation; the digest is untouched by construction.

**Verified.**
- The cap: every row composes under 0.9 m (the C80 test); the tallest drawn are wheat at 0.82 m and rapeseed at 0.81 m.
- `_grass.mjs`: every plot kind's heart grows only its row's grasses; ploughed earth holds no clump (0 of 6,222 in view); a crop planted with `rows` 1 stands on its rows (mean offset 0.00006 of a period) and scattered with `rows` 0 (0.247, a quarter when even); none on the road or under the wood.
- Stations: a low 25 m station per crop on the village (`wheat-25`, `barley-25`, `rapeseed-25`, `hay-25`, `stubble-25`, `pasture-25`, `ploughed-25`) and `wheat-65`.
- Cost: [C81's row](C81-wild-grass.md#outcome). A wheat field fills the play camera with 54,000 clumps against a meadow's 37,000 (it never thins); that station was not cost-measured after the pointer fix.

**Compared** with `brokenarrow/gameplay-trailer-08.jpg`: its fields read as desaturated, textured patches with tree lines; ours now read as crops up close and as striped or grained colour from the play camera, on a palette that is still too saturated and too few (C84).

**Critique.** "Could anything hide a standing soldier": nothing convincingly; wheat and rapeseed look waist to chest high beside the road, enough to hide a crouching man. They are 0.6 to 0.8 m, drawn as designed; the simulation gives no concealment for it (Q-G11), which is the false-buff risk the 0.9 m cap accepts. What did not read as its label on the first pass, and what was done:
- rapeseed read as ripe cereal with blue-grey streaks, barley as green grass: their borrowed palettes were swapped (barley on hay's straw, rapeseed on young crop's green under its yellow heads) and their rows loosened so the stand closes over the gaps;
- wheat and stubble rows stood too far apart on bare orange ground: rows loosened;
- pasture read as seedlings: re-cut wider, lower and flat in colour.

**The second pass**, after those changes: wheat at 25 m and barley read as cereals; hay as standing grass. Still wrong:
- rapeseed has no yellow: on the young crop's green its flower heads come out bright green, a lush lawn. A kind's head colour is relative to its ground, so only a yellow palette makes it rapeseed;
- dark teal streaks between rapeseed's rows read as causeless shadow of the wrong hue: its stems, much darker than its heads, lit by the sky. Not fixed;
- tufts are pale on one side of a frame and near-black on the other in wheat, stubble and barley at 25 m: each blade's own facing against a low sun (`blade_facing`). Not fixed;
- stubble at 65 m is soft stripes with no tufts, and wheat reads as corduroy.

**Open, for C84 and C85.**
- **Palettes.** Rough, prairie, barley, rapeseed, hay and stubble borrow existing palettes (meadow, hay, hay, young crop, pasture, wheat). Wheat's is orange, not gold, and ploughed earth is mauve. A crop's own colour can only come from its plot's palette.
- **Ground.** Between rows and tufts the ground is one flat colour, and ploughed earth is a smooth stripe: the unprimed pass read both as paint with props on it.
- Hay is standing hay; no cut swaths or bales.
- Rows are drawn 1.2 to 1.6 m apart because the terrain's painted rows are; they vanish at grazing angles as the stand closes, which the pass noted.
