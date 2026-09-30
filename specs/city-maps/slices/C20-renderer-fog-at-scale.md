# C20: renderer fog at scale

**Depends on:** C01. **Kind:** slice.

## Question
Does renderer fog cost scale with the occluders in an eye's reach, not with every known occluder?

## Contract it unlocks
The technique S4 picked, inside `frame/fogVisibility.ts` (merge at `:223`). Occluder changes invalidate only the affected eyes (`setOccluders`, `:620`).

## API seam
`packages/battle-renderer/src/frame/fogVisibility.ts`.

## What the human can run or see
`/lab/city-scale` with fog on.

## Verification
- Oracle agreement vectors unchanged.
- Fog scene ≥95%.
- Paired `FOG_COST=1` at G0's occluder count: fog build ≤2 ms.
- The village cost unchanged within noise.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**fog edges at building corners only**) against **the pre-change village fog frame**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Building art (C22+).


## Delegated to the implementer
Bucket or index representation. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Village fog look; spike 02's edge sharpness.

## Feedback that would change this slice
Shadow mistaken for unexplored space reopens fog rendering with physical visibility and light fixed.
