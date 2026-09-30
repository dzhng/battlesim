# C73: tree skeleton

**Depends on:** GG (SG1's budget). **Kind:** slice.

## Question
Do branch-skeleton trees with solid leaf clumps give broken-up crowns within budget and without shimmer (Q-G7)?

## Contract it unlocks
- `trees.py` gains pipe-model radii, gravity droop and parallel-transport bark tubes with furrows (dryad's technique, recorded as a technique entry). Clumps are solid geometry, with no alpha.
- The 4 tiers stay, with the lobed crown as the far tier and the caster tier.
- This slice rebuilds only the 3 existing broadleaf kinds, at unchanged sizes.
- A validator enforces the per-tier triangle budget.

## API seam
`packages/scene-assets/blender/trees.py`, the validator.

## What the human can run or see
A workbench sheet at neutral albedo, and `forest-edge-65` with grass and fog off.

## Verification
- Triangles per tier within GG's budget.
- `fit.*`: the crown stays under the canopy height and radius.
- A frame-cost row at the default and strategic cameras.
- A shimmer check on a 1 px dolly pair.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**crown silhouette only, at neutral colour**) against **`../assets/reference/ground/dryad.jpg` and C62's baseline**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Species, colour, density read, floor.


## Delegated to the implementer
Branch generations; clump size. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
The hedge shrub; benchmark ≥30 FPS.

## Feedback that would change this slice
A branch silhouette that fails at the target camera changes the skeleton parameters before species/material work.
