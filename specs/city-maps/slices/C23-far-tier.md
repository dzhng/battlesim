# C23: far tier

**Depends on:** C22. **Kind:** slice.

## Question
Does the far tier keep each building's massing and roofline, with no visible pop?

## Contract it unlocks
A pure `farTier(parts, height, floors, archetype) → mesh` builder, deterministic, merged per 256 m tile. The tiles draw **through C22's static-chunk owner** (one draw per tile per pass, cast into the far shadow cascades), not a second draw path. The graphs' own LOW tiers are **not** the far tier (they're 47–86k triangles).

## API seam
`packages/battle-renderer/src/city/farTier.ts` (mesh builder only), C22's chunks.

## What the human can run or see
A fixed camera-distance sequence at 150–400 m.

## Verification
- ≤~300 triangles per building.
- Strategic-window GPU and buffer bytes.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**silhouette and roofline across the transition**) against **C22's shot at the transition distance**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Materials, glass, interiors.


## Delegated to the implementer
Tile size; roof detail; transition distance within G0's radius. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Near hero density fixed.
