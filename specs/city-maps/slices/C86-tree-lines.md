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

## Feedback that would change this slice
A tree line that falsely changes movement or sight reopens placement/body fit; colouring alone cannot mask it.


## Outcome — simulation contract (2026-10-01)

The simulation capability already exists in the shared Polygon/Stroke forest
contract and rounded centerline loader; no redundant forest enum or second
placement technique was added. New native battle evidence pairs the same units,
seed and terrain with a strip versus open ground: the far-field recon squad is
identified only in the open arm. A close arm remains identifiable through the
thin line, preserving the ordinary forest attenuation rule. Existing analytic
stroke membership checks prove every trunk lies inside the physical capsule.
A new sparse-foliage export check proves every foliage-cell center is inside a
real trunk's canopy radius, and canopy extends beyond the strip edge.

Both new regressions were falsified: disabling stroke trunk placement made the
far squad visible; clipping foliage to authored forest ground removed the
required over-edge canopy. Both were restored green. Focused forest tests pass;
this pass adds no production behavior and moves no rect forest or village trunk.
Manual shape/diff/docs review and the choices audit found no second owner.

Farmland catalogue/lab construction, plot cuts, fitted drawn crown coverage,
the 40-strip rendered frame-cost row, WARNO comparison, screenshot critique and
visual acceptance remain with the map/visual lane. This is native simulation
verification, not a claim that the rendered hedgerow has passed its gate.
