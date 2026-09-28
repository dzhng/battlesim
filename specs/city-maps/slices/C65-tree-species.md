# C65: tree species

**Depends on:** C64. **Kind:** slice.

## Question
Do trees have broken-up crowns and real variety, all about one size (Q-G7, Q-G8, Q-G8b)?

## Contract it unlocks
- `packages/scene-assets/blender/trees.py` gains a **branch skeleton** (pipe-model radii, gravity droop, parallel-transport tube bark; dryad's technique) with **solid leaf-clump geometry** (no alpha cards).
- Species:
  - spruce and pine (from `~/dev/game`'s conifer parameters, as technique);
  - the 3 broadleaf kinds reworked;
  - birch (from `~/dev/game`'s aspen, as technique);
  - sapling;
  - dead snag;
  - fallen trunk (shared with C67's log body).
- **All about the forest's canopy height and one trunk thickness**; variety is shape and colour only.
- The tiers keep the solid-crown far tier. There's a per-tier triangle budget (L-G6).
- Manifest `technique` entries for dryad and for `~/dev/game`'s `treeCrown.ts` parameters.

## API seam
`packages/scene-assets/blender/trees.py`, `reuse-manifest.json`.

## What the human can run or see
A workbench sheet of every species at 25, 65 and 250 m.

## Verification
- `asset check`.
- Triangles per tier within the budget.
- A frame-cost row on the village woods.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**single-tree silhouettes on the workbench sheet only**) against **`../assets/reference/ground/dryad.jpg` (crown break-up) and the pre-change `woods-models-on` village shot**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Forest mix and density read (C66), floor dressing (C68).


## Delegated to the implementer
Branching parameters within the species' look; leaf-clump density. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Tree placement (one per sim trunk); LOD thresholds unless the budget forces them.
