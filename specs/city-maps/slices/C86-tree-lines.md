# C86: tree lines

**Depends on:** C63, C65, C72, C60 (second in the ground cut order). **Kind:** slice.

## Question
Can a map have tree lines between fields that block sight exactly where they're drawn (Q-G15)?

## Contract it unlocks
- `Forest.shape: Rect | Stroke { points, width_m }` (densified by C65). Rect forests keep identical trunks.
- Trunks are placed along strokes by the one rule. **Crowns clamp to the sim canopy radius, not the shape edge** (`placement.ts:227-231` today), so strips aren't sticks.
- Plots cut along strip control runs. The forest channel in C63 covers strokes.
- Proven on a new catalogued `farmland` lab. **The village gets none** unless the user opts in (then it's a named change).

## API seam
`contract::map::Forest`, `sim::world::forest`, `terrain/plots.ts`, `scenery/placement.ts`.

## What the human can run or see
A sight GIF: a recon squad behind a tree line is unseen from the far field.

## Verification
- Run tweak-mechanics.
- Tests: sight is blocked through a strip; trunks stay inside; drawn crowns are within one fog cell of the foliage cells.
- A frame-cost row with 40 strips.

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**tree-line silhouette along field edges only**) against **battle-look's WARNO hedgerow frames**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Everything else.


## Delegated to the implementer
Strip width; the lab layout. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
The village; rect-forest trunk parity.
