# C85: field texture

**Depends on:** C84. **Kind:** slice.

## Question
Does each field carry ground texture inside it (furrows, clods, stubble) without cloud-shadow blobs?

## Contract it unlocks
Plot-oriented filtered texture in the terrain material, per crop. It's luminance-neutral and one-sided, and fades with pixel footprint like the rows (`terrainMaterial.ts:445-450`).

## API seam
`terrainMaterial.ts`, biome plot rows.

## What the human can run or see
`field-65/250`, grass off, then on at 250 m.

## Verification
- The `fog-look` check again.
- A frame-cost row.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**plot interiors with grass off**) against **`../assets/reference/ground/manor-ground-closeup.jpg`, the Broken Arrow farm frames and C84's shot**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Palette, grass.


## Delegated to the implementer
Texture scales; contrast within the bound. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
C84's mean colours.

## Feedback that would change this slice
Visible tiling or texture that reads as terrain geometry changes field texture scale/blend with palette fixed.

## Outcome

**Built.** `fieldTexture` in the plot term of the terrain material (`frame/terrainMaterial.ts`): what a plot's own ground does to its albedo, per kind from the biome's plot rows.
- **Grain** (`grain_m`, `grain`, `grain_stretch`): four octaves of value noise in the plot's own frame, stretched along its rows, from 7 times a lump's size down to a third of it. Each octave lies on a lattice of its own and is pushed about by the one before; on one lattice, driven into its limits, the grain came out as squares (the first round read as a camouflage print).
- **Rows** (`row_break`): a narrow dark furrow between broad beds (the cube of the cosine the plots had, which alone read as ripples), each furrow broken along its length by its own noise.
- **Wheelings** (`tram`): two furrows bare in every fifteen rows of a drilled crop. The grass build skips them (`groundWheeling`, read by `frame/grassPass.ts`).
- **Dry patches** (`mottle`, `patch_m`): the hue-only patches the plots had, now shaped per kind (strips in a drilled crop, blotches in a meadow) and at half their strength; the grass's dry patches were halved with them. The meadow's bands along its "rows" are gone.

Grain and rows are as much lighter as darker, every value term is finer than about 5 m and fades to its mean under a pixel, and the patches shift hue at the plot's own luminance. The grass takes its colour from the ground under each clump, so it carries the grain.

The plot record on the GPU grew from 48 to 96 bytes. `field_rules.mottle_scale_m` became `mottle_m`. `BattleFrame.setFieldTextureShown` (lab: `suppressFieldTexture`) repacks the plots without texture, for paired frames and cost; the shader skips what a plot does not have.

**Verified** (`web/scenes/_fields.mjs`, `FIELDS_ONLY=1` in the `ground` scene; texture on and off in one page, bare ground, by the class mask):
- every kind of field in view from the play camera carries texture: the luminance spread of 32 px blocks inside a plot, texture on against plain rows, in display levels: meadow 2.6 against 0.03, prairie 2.9 against 0.04, hay 2.9 against 1.5, wheat 3.8 against 2.6, rapeseed 4.9 against 3.6;
- a kind's mean luminance moves by 0.8% at most (the drilled crops, by their wheelings);
- from 250 m no 9 m block is more than 2.3% darker for its texture;
- from 1100 m a pixel moves 0.55 of a display level: it has faded to its mean.
- `fog-look`: every style and framing passes with the texture on (three runs, the last on the final shader).
- `GRASS_ONLY`: every kind still grows its row's grasses (run before the last round's data changes).
- vitest: wheelings are refused without rows, wider than a row, or closer than four rows.

**Frame cost** (texture on against plain rows, 120 forced frames a batch, the median of the paired differences; Apple M-series, the machine shared and loaded):

| Station | Plain, ms | Texture, ms | Pairs |
|---|---|---|---|
| `field-65`, three octaves, turns by `cos`/`sin` | 2.23 | +0.45 | 0.26, 1.60, 0.45, 0.10 |
| `field-65`, three octaves, turns as constants | 2.71 | +0.15 | 0.18, -0.01, 0.15, 0.22, -0.06, -0.47 |
| `field-65`, as shipped (four octaves) | 2.71 | +0.18 | 1.70, -0.02, 0.48, -0.09, -0.12, 0.18 |
| `field-250`, the same three runs | 1.07 to 1.14 | +0.13, -0.02, +0.09 | |

The runs disagree and all are recorded: between nothing measurable and a few tenths of a millisecond where fields fill the play camera. The texture adds five noise lookups to a ground fragment that already made about ten.

**Compared** with `manor-ground-closeup.jpg`, the Broken Arrow farm frame and C84's shots: less wrong, and far from the references. In-field contrast (median luminance spread of 32 px blocks, bare ground at `field-65`) went from 2.2 to about 4 display levels; the references measure 10 to 13, with their shadows, plants and objects in the count.

**Critique** (two unprimed passes; the second after the changes the first caused).

The first, on the three-octave grain with cosine rows: wheat with its rows and wheelings "the only field that fully reads as what it is"; every other field "colour blotches instead of ground structure" (meadow as mouldy felt with rust-orange stains, ploughed earth as blurred ripples or water and low-resolution up close); wheelings read as vehicle tracks at 65 m and as plank seams at 250 m; stubble pinkish, pasture teal, ploughed purple-brown. After it: the fourth, finer octave; furrowed rows in place of the cosine; dry patches halved in ground and grass; four palettes turned toward yellow (a commit of its own).

The second, on the result:
- **works:** stubble at 25 m ("the best of the set"), the patchwork from 1100 m and 2500 m; wheelings read as tracks at 25 m;
- **does not:** ploughed earth still reads as rippled sand (soft bands, no clods), the meadow still as felt with ochre blotches, and from 250 m the crop fields as wood veneer or corduroy, with moiré across the rows of one field;
- no dark region inside a field read as a shadow. The dark meadows beside bright crops in `country-250` read to it as possibly under a cloud (medium): whole fields, the palette's range, not the texture.

**So the slice's question is answered in part.** Drilled and cut crops carry texture that reads as a field. Grass and bare earth carry texture that the checks measure and an unprimed eye still reads as a material sample: value noise on the albedo makes blotches, not structure.

**Open.**
- **Structure for earth and grass.** What is missing is form, not contrast: clods and tussocks as shapes (a cellular pattern), or the furrows' own relief in the shading normal. The second is bounded by the rule that nothing on the ground reads as shadow; neither was tried.
- **Moiré on rows near the horizon** (`ploughed-25`, and one field at `field-250`): the rows fade by the pixel's longest side, so a row seen along its length at a grazing angle beats before it fades. It predates the slice; the narrower furrow makes it no better.
- **Wheelings from 250 m** are hair-thin and ruler-straight. `tram.contrast` at 0 removes them per kind.
- **The warm orange of dry grass** is the grass pass's (`patches.dry`, `dry_lift`), halved here and still read as rust.
- The village scene and the full `ground` scene were not run; `GRASS_ONLY` was not rerun after the last round.

## Current grass continuity feedback

The user asks whether grass-like ground texture can blend the 3D tufts into a continuous surface. The [reported close-up](../assets/reference/ground/reported-grass-2026-10-02.png) is an unapproved defect baseline. Independent investigation confirms the existing ground already has procedural grain and the blades sample its colour. Paired captures isolate much of the dark directional scratch pattern to blade-facing shading; reducing that term preserves more tuft structure than aggressive early softening. Smooth gaps remain. At 250/1100 m no blades draw, so the ground material must carry grass identity. A fresh image-only critic agrees on these limits.

The next bounded prototype belongs jointly to this texture owner and [C81's blade shading](C81-wild-grass.md): replace or refine grass-only fine grain with nondirectional tuft/leaf structure, retaining coarse variation, mean colour and footprint filtering; compare the existing facing contribution separately. Keep density, height, softening curve, palette, crop rows, roads, forest membership and gameplay rules fixed. Reuse `fieldTexture`/`groundColour`, with no second ground layer or downloaded texture. Implementation may select a simple procedural structure and scale inside existing validated tuning; further schemas/resources require a named new contract.

Use the lane's two-station/three-variant/two-round GPU bound. Compare exact paired close/default captures with the reported framing as closely as known, then inspect the final far views and boundaries once. Retain C84 mean-colour, grass exclusion and field/fog checks; measure one short paired cost. Run compare-screenshots, an unprimed complete-set critique and a non-blocking Preview checkpoint before accepting any candidate. Directional hatching, tiled carpet, dissolved tuft depth and motion shimmer are rejection cases. Hills remain deferred.

**Preserve the yellow patches.** The user likes the warm dry-grass variation and its character. Ground and blade colours already share the ground material, with additional world-anchored clump dryness. Further continuity work must retain this variation and the approved blade-facing change.

### Rejected finest-grain prototype

Two bounded rounds replaced only the finest grass-plot grain contribution with tapered leaf marks. The final matched close/default pair changes pixels but does not convincingly blend the ground and tufts: both root and an unprimed critic still see smooth olive gaps close up and weak grass identity at gameplay zoom. The additional shader machinery is rejected and production source is restored. This rejects that hypothesis; grass-ground continuity remains open.

Complete native frames, crops, source snapshots, metrics and the fresh review are preserved in main's ignored `throwaway/grass-surface-prototype/`. The final control/candidate match tick 12 and log no shader warnings. The first round reached tick 13, so its wind-confounded comparison remains diagnostic only. Far/boundary/motion/cost gates were not spent on the rejected candidate and are not claimed. The non-blocking Preview comparison includes the reported defect baseline. A next hypothesis must improve visible ground structure at gameplay zoom while preserving yellow variation, rather than adding finer marks that filter away.

Earlier research remains in root's ignored `throwaway/grass-surface-research/`. Its warning-bearing captures are diagnostic; [C81's integrated checkpoint](C81-wild-grass.md#outcome) owns the approved blade-facing change and warning-free verification.
