# C71: field palette

**Depends on:** C70. **Kind:** slice.

## Question
Do fields read desaturated and textured, like Broken Arrow's patchwork (Q-G13)?

## Contract it unlocks
- The plot palettes move toward olive, tan and brown, with texture inside each field (mottle and crop-row variation at field scale).
- **The `light.shadow_floor` scene check gates it** (L-G4).

## API seam
`fixtures/biomes/summer.json` (`palettes`, `plots`), `terrainMaterial.ts` mottle.

## What the human can run or see
The village at 65 and 250 m.

## Verification
- The shadow_floor scene check.
- Critique: "could any dark region read as shadow, or shadow as fog?"

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**field colour and texture across the patchwork only**) against **battle-look's Broken Arrow frames and the pre-change `frame-tanks-far` village shot**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Roads, forests, grass blades.


## Delegated to the implementer
Exact palette values within the direction. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Plot layout.
