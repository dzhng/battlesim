# C26: interiors

**Depends on:** C25, C15. **Kind:** slice.

## Question
Do rooms behind windows reproduce the repo's mechanism exactly, unlit, within budget?

## Contract it unlocks
Room meshes plus the flat-perspective atlas lookup with stable per-instance room selection by position hash, on every floor of the tiers G0 allowed (O-1). Ground floors show shops. **Emissive zero.**

## API seam
`packages/battle-renderer/src/models/`, the C21 interior metadata.

## What the human can run or see
A facade fixture at 30 and 80 m, and a street-level storefront.

## Verification
- Stable selection under camera movement.
- Frame-cost row within budget.
- Record the O-1 verdict (all room tiers, or LOD0 only).

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**window interiors only: "does any window read as lit, or as a hole?"**) against **the Blender render of the same facade (S2 driver, our atlas swapped in)**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Facade materials, massing.


## Delegated to the implementer
Nothing beyond the repo's mechanism (Q-E). Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Glass unchanged; no walkable interiors.

## Feedback that would change this slice
If the user wants rooms to read darker or lighter, tune the atlas (C15), not the shader.
