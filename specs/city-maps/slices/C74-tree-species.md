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

## Outcome (2026-10-01)

**Contract as landed.** `trees.py` grows seven tree kinds from one generator: the three broadleaves, and now `tree_spruce`, `tree_pine`, `tree_birch` and `tree_snag` (catalogue entries, sources and bundles). A species is a row of parameters and, for a conifer, another arrangement of the same clumps:

- **Spruce:** boughs in whorls up one straight leader under a spire, inside a tapering profile (`~/dev/game`'s conifer profile, as technique). Its far tier is that outline turned round the leader, because a lobed volume cannot follow a spire from one centre.
- **Pine:** a broadleaf crown of ten flat pads high on a bare bole, the upper bark orange.
- **Birch:** a narrow crown of small hanging clumps on a white bole (`~/dev/game`'s aspen proportions, as technique).
- **Snag:** a tree's skeleton with no clumps, every tier bark alone, its branches ending in points.

The validator holds every tree to one size (`fit.tree_size`, the size on `SCENERY_KINDS.tree`): top 11 m and bole 0.40 m in radius at breast height, each within 5%. The bole is measured as the area the bark encloses at 1.3 m, so a leaning or furrowed trunk has the girth it has.

**What changed from the slice as written.** The spreading and tall broadleaves were rebuilt to enter the band (10.0 → 10.5 m, 11.8 → 11.5 m, the tall kind's bole 0.32 → 0.40 m). The common broadleaf and the hedge shrub are byte-identical to C73's.

| Kind | Triangles per tier | Top | Reach | Bole |
|---|---|---|---|---|
| budget | 10,000 / 2,500 / 500 / 80 | 10.45 to 11.55 m | ≤ 6.5 m | 0.38 to 0.42 m |
| broadleaf | 9,312 / 2,432 / 500 / 80 | 11.00 | 5.90 | 0.398 |
| spreading | 9,312 / 2,432 / 500 / 80 | 10.50 | 6.40 | 0.405 |
| tall | 8,184 / 2,132 / 440 / 80 | 11.50 | 3.69 | 0.399 |
| spruce | 8,796 / 2,276 / 500 / 78 | 11.20 | 5.30 | 0.403 |
| pine | 4,338 / 1,120 / 220 / 80 | 11.00 | 5.53 | 0.396 |
| birch | 9,312 / 2,432 / 500 / 80 | 11.30 | 3.38 | 0.401 |
| snag | 1,134 / 382 / 68 / 22 | 10.71 | 3.03 | 0.391 |

**Shimmer** (C73's rotation pair at the village's wood edge, a wood of one species against today's wood): birch 1.29×; spruce 2.10× at first, over the 2× bar. The residual sits on silhouette edges, and the first spruce was thin plates with gaps between its whorls. Rebuilt fuller (wider boughs that close each whorl and hide the one above) it measured 1.69×.

**Compare.** Against C73's sheet the four new columns each have a silhouette of their own from the front; against `forest-road-autumn.jpg` the conifers share its bare boles and tiered, drooping boughs, in far coarser masses (24 clumps a crown is the budget). Less wrong than one crown shape; not a match.

**Critique** (unprimed, on the first sheets). No dark region read as the unseen overlay. Findings and what was done:

- Broadleaf, pine and birch were one green ball from above: the pine was rebuilt as fewer, separate pads and the birch narrower with smaller clumps. They still part mostly by colour and bole from straight above (C75's tints carry that).
- The pine's crown was a flat slab with cut edges: its clumps now stand less proud of the crown, so fewer are brought back onto its reach.
- The spruce was separate plates with the trunk showing: rebuilt fuller, as above.
- The snag's forked limbs read as propellers: it is now a pole with short stubs. It is thin at the far tier (22 triangles).
- Left: no fill light on a crown's shaded side (the workbench's light, C73 found the same); the birch's bark marks do not show at the sheet's size; trunks are plain poles; crowns are lumpy.

A second critique was not run on the rebuilt sheets; C75's critique saw the rebuilt kinds in the wood.

**Changed again by C76** (the table is as shipped): the spruce and pine were widened to close the canopy (reach 4.6 → 5.3 m and 4.9 → 5.5 m), the spruce's crown base lifted from 2.0 m to 3.6 m, the pine's limbs kept under its pads and the birch's bole dulled. The sheets below predate those.

**Pictures** (scratch): `throwaway/sheets/c74b/species.png` (one column a species, C73's broadleaf first), `throwaway/sheets/c74b/<kind>/contact.png`; the first round in `throwaway/sheets/c74/`.
