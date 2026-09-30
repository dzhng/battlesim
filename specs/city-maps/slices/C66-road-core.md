# C66: road core

**Depends on:** C65, C62. **Kind:** slice.

## Question
Does each road kind's core read as that kind, not a grey blob?

## Contract it unlocks
Per-kind core palette, mottle and roughness: country road is gravel or asphalt grey; dirt track is packed warm brown. This replaces `palettes.road`. The core's luminance stays at or above the grass's (L-G3).

## API seam
`terrain/biome.ts` road rows, the `terrainMaterial.ts` road term.

## What the human can run or see
`bend-25/65` and the lab's dirt-track stations.

## Verification
- A mask-based numeric check: core luminance ≥ the adjacent grass band's.
- Digests untouched.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**core interior only (SD < −0.5 m)**) against **`../assets/reference/ground/forest-road-summer.jpg`, `manor-path-closeup.jpg` and C65's shot**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Shoulder, ruts, grass at the edge.


## Delegated to the implementer
Palettes and mottle. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
C28/C31's city pavement rows.

## Feedback that would change this slice
A road core that disappears at tactical distance changes the core read with width/geometry fixed.
