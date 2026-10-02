# C76: canopy closure

**Depends on:** C75. **Kind:** slice.

## Question
Does a forest read dense but let you see units and the floor beneath (Q-G16)?

## Contract it unlocks
Crown scale is tuned so the canopy mostly closes. **The crown radius never exceeds the sim's `canopy_radius_m`.**

## API seam
`summer.json` (`trees.forest`), `placement.ts`.

## What the human can run or see
The default camera and 120 m, with one test squad inside.

## Verification
- The visible floor share from a models on/off pair.
- The test squad's torsos visible above a threshold (the `torsos` helper in `_battleLook.mjs`).

**Visual verification.** Do these in order; each is required:
1. Freeze the camera, light, seed and every variable except this slice's own.
2. Run [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md) on the crop or mask (**forest interior at 65 m; critique: "can you see the soldiers under the trees?"**) against **battle-look's Broken Arrow forest frames (target: looser than theirs) and C75's shot**. It gives telemetry and a less-wrong verdict, not a match check.
3. **Last, before you accept the shot,** run an unprimed [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md). Include the question "could any dark region read as shadow, or shadow as fog?"
4. Non-blocking checkpoint: open the shots with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md) and allow about 5 minutes. If the user is silent, decide on the evidence, record the decision and why in this slice's section of `../choices.md`, close the shots and proceed.

**Out of scope for this shot (later slices):** Floor dressing, bodies.


## Delegated to the implementer
Crown scale within the radius. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
The one forest rule.

## Feedback that would change this slice
Visible canopy holes change closure parameters while preserving species silhouettes and the one forest rule.

## Outcome (2026-10-02)

**Contract as landed.**

- **The radius.** `trees.forest.girth` is a tree's drawn width as a share of its appearance's own, and the biome refuses a value over 1. The asset validator already holds every appearance inside `forests.rule.canopy_radius_m` (`fit.canopy`), so no drawn crown passes the simulation's radius and the number keeps one owner. A placement test checks every village crown against the fixture's radius. Before this slice crowns reached 6.95 m against 6.5 m.
- **The edge.** The clamp to a crown's room inside the forest's shape is deleted: a tree at a wood's edge is as wide as one inside (C73's blade-thin trees), and its crown overhangs the field as the simulation's foliage does.
- **Closure.** `girth: [0.9, 1]`, with the spruce and pine built wider (5.3 m and 5.5 m in reach).
- **The check.** `web/scenes/_canopy.mjs` (in the `ground` scene; `CANOPY_ONLY=1` alone): the share of the wood's floor pixels still seen with the trees drawn, from the terrain's own class mask, must lie between 0.12 and 0.40 at the new stations `forest-65` and `forest-120`.
- **The floor.** `forestFloor` drifts moss and humus on turned noise; `forest_floor.moss` and `.humus` are their strengths.

**What changed from the slice as written.** The seam is the biome's `girth` and the art's reach, not a radius read in placement: the canopy radius does not reach the renderer and this lane may not change `crates/`. The squad-torso check is not in a scene (below).

| Measure | Before (C75) | Landed |
|---|---|---|
| Floor seen through the crowns, 65 m / 120 m | 0.32 / 0.30 | **0.23 / 0.22** |
| Widest drawn crown | 6.95 m | 6.40 m (canopy radius 6.5 m) |
| Narrowest drawn tree at a wood's edge | a few centimetres | 0.9 of its appearance |
| Bare floor: edge density, darkest twentieth (of 255) | 0.12, 69 | under 0.02, 83 |
| Shimmer, the mixed wood against today's | | 1.16× (bar 2×) |

Girth alone was not enough: at 0.84 to 1 with the first conifers the floor seen was 0.33 / 0.31, more than before, because the conifer stands stayed open. Widening the conifers' art closed them.

**Not met: a test squad's torsos.** In the village scene's woods tour the squad stops at the wood's corner under a spruce and a broadleaf: **0 of 8 torsos are seen plainly; all 8 are drawn by the x-ray.** It was 1 of 8 before the spruce's skirt was lifted from 2.6 m to 3.6 m, which did not help. Before this slice those corner trees were blade-thin. One squad is one sample, so no torso check was added to the scene; a fair one needs many positions. The honest reading: under a canopy this closed, units are found by the x-ray, not seen directly. Opening the wood to show them means giving up "mostly closes". That choice is the user's.

**Frame cost.** Paired with today's trees at the village's wood edge, trees on and off in each arm (5 rounds, 150 forced frames, Apple Metal, 1920×1080): this slice's arm read **−0.61 ms** against today's (rounds −1.16 to +0.50), and trees on against off was inside ±0.7 ms in both arms. C75's run read +0.50 ms on the same station. The machine was loaded; the two runs disagree and neither is near the 1.5 ms bar. Not resolved further (one run a slice).

**Compare.** Against C75's shots at the same stations: mean luminance within 3.5 of 255, the floor lighter and smooth. Against the Broken Arrow frames: denser than before and still looser than theirs, as Q-G16 asks; ours shows bare brown floor where theirs shows undergrowth (C79).

**Critique** (unprimed, nine shots, before the last changes). The floor is "not blocky or tiled". Findings and what was done:

- The wood reads as an open plantation at 65, 120 and 250 m, "never a solid lid and never a dense wood": a third of the ground showing by its eye, trees in even rows. The rows are the simulation's trunk grid (9 m, 0.3 jitter). Closure was left at 0.23: closing further hides the units the same slice wants seen. Open, with the torso result above.
- All eight soldiers are pale outlines, findable only if you know where to look: the x-ray's contrast is not this lane's.
- Brown branch stubs above the pines' crowns: the pine's limbs now leave the bole level, under its pads.
- Cream-white birch boles read as pegs: the bole's colour was dulled by a third.
- Dark blotches on the floor read as a painted pattern, not cast shadow (the sun-fleck dapple and crown shadows together); shadows on wheat read grey at one station and orange at another. Not changed: the dapple and the grass's shading are not this slice's.
- Bare floor with no undergrowth, a hard edge with no shrubs, thin root lines as hairline arcs: C79's dressing and the verge.
- Woods are axis-aligned rectangles on the village (map data); at 1,100 m a wood is speckle.

The final shots were taken after those changes and not critiqued again.

**GPU.** This slice took three holds of the lock, one over the budget's two rounds: two variants on three stations; the candidate's full set, which the critique sent back; the final set.

**Pictures** (scratch): `throwaway/shots/c76f/` (final) against `throwaway/shots/c75/`; `throwaway/shots/c76f/woods/woods-xray-1920x1080.png` (the squad).
