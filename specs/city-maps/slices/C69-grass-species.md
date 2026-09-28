# C69: grass species

**Depends on:** G0. **Kind:** slice.

## Question
Does grass come in species presets that vary within a field (Q-G10, Q-G14)?

## Contract it unlocks
- Species presets (blade shape, height ≤0.9 m, width, colour ramp, clumping, wind response) as clump-model variants from our generator (`scene-assets/src/grass.ts`).
- Wild ground: short meadow, tall rough grass, dry prairie, weedy verge.
- Variation within a field: colour, height and clump noise.
- **Measure first, then cap species on screen per biome** (for example 4, L-G7).

## API seam
`packages/scene-assets/src/grass.ts`, `frame/grassPass.ts`, `terrain/grassField.ts`, the biome `grass` section.

## What the human can run or see
The village meadow at 25 and 65 m.

## Verification
- Grass ms at ground and default cameras.
- Draw count.
- Critique.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**a meadow patch only**) against **`../assets/reference/ground/grass-a.jpg`, `grass-b.jpg` and the pre-change `grass-traverse-25` village shot**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Crops (C70), field colours (C71).


## Delegated to the implementer
Preset parameters; the species cap within the measurement. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Grass never grows on roads, water, forests or prop footprints.
