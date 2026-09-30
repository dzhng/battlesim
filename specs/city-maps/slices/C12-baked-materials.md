# C12: baked materials

**Depends on:** C11. **Kind:** slice.

## Question
Do the graphs' materials, baked into our three texture slots, keep the facades' character?

## Contract it unlocks
Materials bake into albedo, normal and ORM with box-projected UVs, the grime burned in (Q-H′), at the kit texture edge G0 chose, in the array G0 chose. This never inflates the shared array (L8).

## API seam
`packages/scene-assets/blender/city/`.

## What the human can run or see
An 80 m facade contact sheet per archetype.

## Verification
- `asset check`.
- Texture bytes.
- No false wear from tint alpha (L9).

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**facade surface only: brick, stone, grime; exclude windows, sky and ground**) against **`throwaway/evidence/city-maps-chiro/sheet.jpg` (80 m column; regenerate it with `specs/city-maps/assets/render_rts_views.py` if it is missing)**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Glass, interiors, massing, far tier.


## Delegated to the implementer
Texture packing within the edge. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Geometry, lighting, glass and interiors stay fixed.

## Feedback that would change this slice
A material that loses wear/tint or scale changes its source bake while retaining the transport channel contract.
