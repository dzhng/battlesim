# C80: grass presets

**Depends on:** GG. **Kind:** slice.

## Question
Can every grass and crop preset guarantee its final drawn height ≤0.9 m, and carry shape, colour and wind response (Q-G10)?

## Contract it unlocks
- `GrassSpec` gains blade shape, width, colour ramp, clumping and a wind-response factor (it scales the shared wind; no new clock).
- **A validator for effective field height ≤0.9 m that includes every biome scale and variation multiplier.** Today's wheat (0.95 m, `assets/catalog.json:664-667`) makes it fail first, then it's fixed.
- Blades stay ≤`GRASS_MAX_BLADES`. Measure the padding cost (every clump is padded to 8 blades).

## API seam
`packages/scene-assets/src/grass.ts`, the grass schema, the validator.

## What the human can run or see
A workbench grass sheet with a 0.9 m ruler.

## Verification
- Validator red, then green.
- Adversarial multiplier test.
- Generator determinism.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**blade and clump shape at uniform tint**) against **`../assets/reference/ground/grass-a.jpg`, `grass-b.jpg` and today's sheet**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Colour, fields.


## Delegated to the implementer
Packing; neutral defaults. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
One indirect draw per tier; capacity bounds.
