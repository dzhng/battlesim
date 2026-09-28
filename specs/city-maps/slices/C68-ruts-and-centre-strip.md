# C68: ruts and centre strip

**Depends on:** C67 (first in the ground cut order). **Kind:** slice.

## Question
Do ruts and a dirt-track centre strip add close detail without a ghost outline at 65 m?

## Contract it unlocks
- Two noise-broken tracks inside the core, sunk by a **normal tilt of ≤15°** (the terrain is never displaced).
- They fade to the core's mean below about 2 px per rut, using the same footprint fade as the plot rows (`terrainMaterial.ts:445-450`).
- On dirt tracks below a width threshold, grass grows in a centre strip, keyed from the field.
- SG3's verdict can reduce this to roughness and normal only, or cut it.

## API seam
`terrainMaterial.ts` road term, `grassPass.ts` growth.

## What the human can run or see
`bend-25`, then `bend-65/120/250` (to show the fade), and the lab's `track-25/65`.

## Verification
- At `bend-250` the core's variance is within 1% of C67's; at `bend-25` the ruts are measurably present.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**the road core only; critique asks "does the road read as an outline?" at 65 m**) against **`../assets/reference/ground/forest-road-rocks.jpg`, `forest-road-summer.jpg` and C67's shot**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Shoulder, grass outside the core.


## Delegated to the implementer
Rut spacing and darkness (bounded by the shadow floor), the centre-strip threshold. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
C67's shoulder.
