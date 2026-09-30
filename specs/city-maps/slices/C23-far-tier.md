# C23: far tier

**Depends on:** C22. **Kind:** slice.

## Question
Does the far tier keep each building's massing and roofline, with no visible pop?

## Contract it unlocks
A pure deterministic far-tier builder from reusable descriptor geometry/roof recipes, assembled at load/runtime through C22's bounded chunks. It needs no per-map Blender bake. Tile sizes and any very-far aggregation follow S3/G0's full-overview verdict; 256 m is an initial experiment, not a fixed full-map allocation. The tiles draw **through C22's static-chunk owner** (batching and far shadow casters follow S3/G0's measured limits), not a second draw path. The graphs' own LOW tiers are **not** the far tier (they're 47–86k triangles).

## API seam
`packages/battle-renderer/src/city/farTier.ts` (mesh builder only), C22's chunks.

## What the human can run or see
A fixed camera-distance sequence from the near/far transition through the full Large overview, with matched runtime/saved inputs.

## Verification
- Per-tier triangle/instance budgets from S3/G0; the earlier ~300 triangles per building is only an initial far-tier target and cannot substitute for whole-map overview cost.
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

## Feedback that would change this slice
Distant buildings that pop or lose town character change far-tier thresholds/representation within G0 budgets.
