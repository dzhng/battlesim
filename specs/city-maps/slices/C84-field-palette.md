# C84: field palette

**Depends on:** C83, SG4. **Kind:** slice.

## Question
Do fields read desaturated (olive, tan, brown) while seen ground never reads as fog (Q-G13)?

## Contract it unlocks
The plot palettes in `summer.json` move to GG's palette, with rapeseed yellow, plus the biome L* floor test from SG4.

## API seam
`fixtures/biomes/summer.json` (`palettes`, `plots`).

## What the human can run or see
`field-250` and `patchwork-1100`.

## Verification
- **The `fog-look` darkest-seen check passes under every style** (L-G4).
- The L* floor test.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**plot albedo across the patchwork, excluding roads and forests**) against **battle-look's Broken Arrow farm frames, `../assets/reference/ground/ours-vs-refs-board.jpg` row 4, and C62's `patchwork-1100` baseline**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** In-field texture.


## Delegated to the implementer
Palette entries within the floor. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
`fog-look`.

## Feedback that would change this slice
Fields that merge into shadow or fog change field palette while retaining accepted light, density and geometry.

## Outcome

**Built.** Every plot kind has a palette of its own in `fixtures/biomes/summer.json`: olive meadow, pasture and rough, khaki prairie, gold wheat, green-straw barley, yellow rapeseed, hay, pale stubble, brown ploughed earth. The borrowed palettes and `young_crop` are gone; the verge and the distant land moved with them. Colours were drafted in CIELAB (lightness, chroma, hue) and stored as sRGB.

The ground round the houses is a plot kind of its own, `green` (`field_rules.settlement_kind`), on the meadow's former colours and growing what the meadow grows: [SG4](SG4-palette-vs-shadow-floor.md#outcome) found the fog check held there by a hair, and a desaturated meadow under it fails.

**The floor.** `validateBiome` refuses a plot kind whose darkest ground is under L\* 24 (`PLOT_MIN_LSTAR`, `darkestPlot`, `terrain/biome.ts`): its palette's darkest colour at the low end of the per-plot jitter, rows at their mean. A vitest holds a colour on the floor, the same colour under it, and the same colour under deep furrows.

**Numbers.**
- `fog-look`: every style and framing passes, each number within 0.7 of main's on the same day (the settlement's ground is main's).
- Plot pixels on screen, by the class mask (roads and woods out), CIELAB chroma, median: `patchwork-1100` 32.5 → 20.7, `field-250` 39.3 → 21.6. The Broken Arrow farm frame's fields: 12 to 16. Mean lightness went 44 → 49 (the reference: 30 to 35, under an overcast sky).
- The low sun adds about 10 of b\* to anything it lights, so an albedo of chroma 8 to 17 reads at 20 on screen; the palette is drafted that far under what it should read as.

**Compared** with `brokenarrow/gameplay-trailer-08.jpg`, board row 4 and C62's `patchwork-1100`: less wrong (a muted patchwork in place of lawn green and orange), still about half again as chromatic as the reference and lighter, with a narrower range of value.

**Critique** (one unprimed pass, on `patchwork-1100` and `field-250`). The fields read as muted olive, grey-green, tan and brown farmland; nothing reads as too dark, and no dark field reads as a shadow (each stops at its boundary). What it found, and what was done:
- brown leaning mauve, tan leaning peach, sage and khaki merging: ploughed earth turned toward yellow-brown, wheat toward gold, stubble paler, barley greener (not shot again on their own: C85's stations carry them);
- the green lines between fields more vivid than the fields: the verge was desaturated with them;
- **the block of lawn green round the buildings, saturated against everything else, with a hard edge: open.** It is the settlement's `green`, held by SG4;
- rapeseed yellow pulls the eye first: wanted (Q-G10), left;
- an even grey-green veil toward the frame's edges read as haze: the rig's own aerial haze at 1100 m, not the palette.

**Open.**
- The settlement's ground waits on the fog styles' margin (SG4's last paragraph).
- No pale cream field and no really dark green: the floor and the sun's warmth bound the range from this side.
- Six kinds are told apart from 1100 m, not ten; texture (C85) separates the rest closer in.
