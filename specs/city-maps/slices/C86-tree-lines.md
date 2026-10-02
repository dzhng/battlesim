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

## Outcome — drawn half (2026-10-02)

**Changed since.** The open-country pass ([`choices.md`](../choices.md), "Open country") writes tree lines as stroke forests on generated maps, so the picture, the hedgerow comparison and the cost row this Outcome waits for can now be taken on one. There is still no `farmland` lab.

**No map has a tree line.** No saved map holds a stroke forest and the generator makes none, and the `farmland` lab is the map lane's. So the drawn half is held by tests on an inline map (a strip 10 m wide, bent once) and has no picture: no hedgerow comparison, no critique, no 40-strip frame-cost row. Those wait for the first map with a strip.

**What is true of a drawn tree line, and where it is held** (`web/tests/scenery.test.ts`, `web/tests/surfaces.test.ts`):

- **Crowns.** A tree in a strip is as wide as one in a wood: nothing narrows it to the strip. This was already so after [C76](C76-canopy-closure.md#outcome-2026-10-02), which deleted the clamp to the shape's edge and holds every appearance inside the simulation's canopy radius (`fit.canopy`); the test here is new. The crowns overhang the fields either side, as the simulation's foliage does.
- **Foliage.** Every point on a drawn crown's rim lies in a fog cell with foliage or beside one, and every foliage cell has a drawn tree within the canopy's radius of its centre.
- **Plots.** Fields are cut along a strip's long stretches (`forestStripRuns`, read by `generatePlots` beside the roads' and rivers' runs), so a tree line stands on the boundary between two fields. The slice says "control runs": the simulation exports none for forests, so a run is any stretch longer than the strip is wide, which keeps the straights and drops the chords of a bend (under 2 m each). No map's plots moved: none has a strip.
- **Floor and dressing.** The forest floor already covers strokes (C63). The floor's dressing keeps 3 m inside a forest's edge, so a strip 10 m wide carries a 4 m band of it.

**Open.** The picture; the hedgerow comparison; the cost of 40 strips; whether a strip narrower than a crown reads as a hedge or as a row of park trees. A forest control-run export would replace the stretch-length rule.

## Outcome — drawn half, first picture (2026-10-02)

**The rig stands on a tree line.** Two generated-map stations, `tree-line-250` and `tree-line-65` (`web/scenes/_groundStations.mjs`, `treeLine`): the middle of the longest straight strip of forest, 80 m or more, that a unit finds wooded along its middle and open ground 4 m and 30 m beside on both sides, at least 400 m inside the map, looked at across. On the rig's map (mixed, medium, seed 2) that is the strip from (3359, 6213) to (3353, 6312): 100 m long, 12 m wide, about fifteen trees. That map has 11 strips, five of them 80 m or longer; seeds 1, 3 and 4 have 23, 12 and 19. A map with none leaves the stations with nothing to stand on, and `shoot` refuses them.

**What the picture shows** (final, bare ground and class mask at both heights; one unprimed critique on the two final shots):

- **It is a row of trees, not a hedge.** The critique calls it "a windbreak or row of trees", "isolated and placed", "an acceptable tree line but a thin one". Separate crowns with a crown's width between some of them, no understorey, nothing continuous.
- **It does not look like it blocks a view.** "A porous screen, not a sight-blocking barrier": bare ground shows straight through between the trunks, and only one two-deep clump looks opaque. The simulation hides a far squad behind it, so the picture promises less than the rule delivers. This is the slice's own question, and the answer is no.
- **Broadleaf trees are lollipops close up**: a crown on a straight, bare, leaning pole, one of them a saturated maroon. Conifers read as stacked cones, very dark on their shaded side, and show no trunk or contact with the ground.
- **Fields stop at it.** Both long sides end at the strip, which stands on the boundary between two fields, as the plot cut intends. It overshoots the field corners at both ends and stops as a blunt blob, its full width of floor against a thin field margin, with no taper.
- **The floor is the loudest part.** A scalloped brown and olive strip that reads as "bare mud" or a decal up close and "a brown smudge or scorch mark" from 250 m. Its dark blotches and the trees' shadows on it cannot be told apart. Its edge is soft against the sharp wheat, and two tramlines seem to run a little way under it (low confidence).
- **Shadows.** Every shadow has a tree to cast it. The ones on the wheat read as tree shadows. One large soft dark patch on the green field at the right of the close shot, whose trees are partly out of frame, could be taken for a tinted overlay (medium-low confidence).
- **Specks.** The floor's dressing reads as stray dark green blobs and tufts too small to register, not as bushes.

**Nothing was changed.** None of this is an error in trunk placement or in the plot cut; it is the look of a strip one crown wide, and the tree models'.

**Open.** A tree line needs a body below the crowns (an understorey or shrub layer along the strip) before it reads as a view-blocker; the floor under a strip wants to be a verge, not a wood's bare earth, and to taper at its ends; trunks. The hedgerow comparison and the cost of 40 strips are still not done. No station looks along a line or from the ground.
