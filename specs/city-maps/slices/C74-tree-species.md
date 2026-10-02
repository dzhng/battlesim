# C74: tree species

**Depends on:** C73. **Kind:** slice.

## Question
Do new species vary in shape while sharing one size (Q-G8, Q-G8b)?

## Contract it unlocks
- Spruce and pine (`~/dev/game`'s conifer parameters, as technique), birch (its aspen parameters, as technique), and a standing snag.
- All within ±5% of one height and girth.
- **Saplings are dressing (C79), and fallen trunks are log bodies (C77)**, not tree kinds.

## API seam
`trees.py`.

## What the human can run or see
A workbench sheet, one column per species.

## Verification
- The validator's size envelope.
- Tier budgets.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**per-species silhouette at neutral colour**) against **C73's sheet and `../assets/reference/ground/forest-road-autumn.jpg`**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Colour, mix.


## Delegated to the implementer
Branch and clump arrangements. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
C73's broadleaf.

## Feedback that would change this slice
An indistinguishable species silhouette changes its parameters while preserving the shared skeleton/export contract.
