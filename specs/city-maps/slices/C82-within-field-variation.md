# C82: within field variation

**Depends on:** C81. **Kind:** slice.

## Question
Does a field vary in colour, height and clumping instead of reading as a mown carpet (Q-G14)?

## Contract it unlocks
- **A growth row becomes a weighted mix of appearances**, and a clump picks its species by hash at build (`grassPass.ts:334-349`). This removes the one-appearance-per-row limit.
- World-anchored low-frequency height, density and colour fields.
- Colour varies in **hue and saturation at bounded luminance**, one-sided (no two-sided blobs, per the avoid-list).

## API seam
`grassPass.ts`, the biome growth rows.

## What the human can run or see
`field-65/250`.

## Verification
- Luminance spread inside a plot stays under a bound while hue spread rises.
- Retained roots are stable under zoom.
- A grass-build ms row.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**one meadow plot's interior; critique: "could any patch read as cloud shadow?"**) against **`../assets/reference/ground/manor-ground-closeup.jpg`, `grass-b.jpg` and C81's shot**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Crops, palette.


## Delegated to the implementer
Noise scales; amplitudes. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
C81's species.

## Feedback that would change this slice
Patchiness that reads as random noise changes within-field variation while retaining field boundaries and height caps.


## Outcome

**Built.** The biome's growth row (`GrassGrowth`, `terrain/biome.ts`) is now:
- **`mix`:** one to four grasses (`GRASS_MIX_MAX`), each with a `share`, a `drift` (how far it gathers in patches of its own instead of spreading evenly) and a `dry` (how far its clumps stand toward straw). A clump is one of them by its own hash against the shares, in the build (`frame/grassPass.ts`).
- **`patches`:** how the stand varies across a field in world-anchored patches: `height` (a range of scales), `thin` (how far sparse patches thin), `dry` (how far dry patches shift toward straw). Their length scales are `grass.patch_m`.
- Each clump is also a little lighter or darker (`clump_value`), half on its own and half with the tussock it stands in (`patch_m.grain`).

Everything is a function of the clump's tile, rank and place, so a clump the camera keeps stays the grass it was. The tallest patch is the row's now, so the 0.9 m cap composes it per grass and per row. Two uniform arrays hold the rows; no binding was added.

**Verified** (`_grass.mjs`, each arm planted through `retune` so the biome's numbers can move):
- three to one planted, three to one grown: 0.737 of 1,337 clumps;
- a dried field against the same field level, clump for clump over 36,985: none darker (-0.1%), none more than 13% lighter with a 10% lift asked (the rest is the colour's byte rounding), none cooler, 44% dried;
- the grain: no clump further than 11.4% from the ground's brightness with 10% asked;
- closer on the same ground (65 m to 40 m): 17,475 clumps kept, none changed kind;
- vitest: shares pack to one, a tall grass in a mix is refused only while the row's patches lift it past the cap, a mix is one to four.

**Grass-build cost:** the moving-camera column of [C81's row](C81-wild-grass.md#outcome) regrows every frame: 1.45 ms at `field-65` against 1.69 before the pass.

**Critique, and what changed for it.** The first unprimed pass saw the meadow vary in colour but not in height or density, and called its soft dark bands and blobs cloud shadow or stains (`meadow-65`, `field-65`, `country-65`), and its dry blades confetti. The patches were one-sided, but 11 to 17 m across and lighter than their field, so what was left between them read as the dark shape. After it:
- patches are 3.5 to 6 m across (texture, not weather), sparse patches thin less, and drying lightens by 8% in place of 20%;
- a clump's dryness and grain fade out with the clump, so the field's far edge meets the painted ground in the ground's colour (the pass saw a saturated band there);
- softening is complete by the fade's start.

**The second pass**, on the changed field: the smoky blobs are gone from its list; the meadow at 25 m "reads as real grass" with a "believable" dry patch. What it still saw:
- screen-vertical darker and lighter columns in `meadow-65` and `country-65`, read as shadow bands or mowing stripes. They are the terrain's own dry strips along the plot's rows, under the grass (the plot term, C85's), not the grass;
- soft yellow pools in barley and hay that read as sun through cloud: the dry patches again, now small. Accepted and recorded, not tuned a third time;
- bald flat spots at 25 m where a sparse patch shows the ground, and ochre flecks at 65 m that read as noise.

**Open.**
- The dry patches are provisional: twice an unprimed eye has read them as light or weather. If the user reads them so too, `patches.dry` goes toward 0 and the variation is left to the species mix.
- Hue varies and brightness barely does, by design; whether that is enough "not a mown carpet" at 65 m is a call for the user on the pictures. More would need the ground under the grass to vary with it (C85 can read the same fields: `groundTint` is exported).
