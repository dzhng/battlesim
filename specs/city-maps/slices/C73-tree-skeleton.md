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

## Outcome (2026-10-01)

**Contract as landed.** `trees.py` grows a skeleton to solid leaf clumps (trunk, a limb per cluster of clumps, a branch per clump; pipe-model radii, beam droop, parallel-transport tubes, a furrowed bole on the finest tier; dryad's technique, recorded in the script's header). No alpha anywhere. The three broadleaf kinds are rebuilt at their old top and reach (11.00 × 5.90, 10.00 × 6.20, 11.80 × 3.78 m); the hedge shrub's source is byte-identical. The validator refuses a tree tier over its triangle budget (`budget.tier_triangles`, the budget on `SCENERY_KINDS.tree`) and a tree that passes the canopy's height or radius on any tier (`fit.canopy`).

**What changed from the slice as written.** Clumps are drawn on tiers 0 to 2, not 0 to 1, and the far tier is the lobed volume of the tree's own clumps, not the old seeded crown. The reason is the verdict on shimmer below: a tree's shadow is cast by the tier under the one drawn, and a caster of another shape shadows the crown it stands for.

| Question | Verdict |
|---|---|
| Do skeleton trees with solid clumps fit the frame? | Yes, at about 24 clumps a crown: +0.50 ms GPU (median of 6 paired batches, −0.31 to +0.98) where the wood fills the frame at the default distance; the same triangle counts read +0.96 ms on a busier machine. The bar is 1.5 ms. |
| Tier 0 under 25k triangles? | Yes: 9,312. |
| Shimmer no worse than 2× today's? | Yes: 1.05× (0.123 against 0.117). It was 3.2 to 3.4× while the lobed crown cast for the clumps, whatever their number or contrast. |
| Broken-up crowns? | Partly. Crowns are separate lumped masses with crevices and a broken outline, and limbs show from below. They are not open: 50 small clumps with sky between them cost +1.6 to +1.9 ms and were dropped. |

**Triangles per tier** (broadleaf and spreading; tall in brackets): 9,312 / 2,432 / 500 / 80 (8,184 / 1,880 / 440 / 80). Today's were 5,188 / 1,328 / 340 / 80. **Budget: 10,000 / 2,500 / 500 / 80.**

**Frame cost.** Paired, today's trees against the new, interleaved in two pages on one dev server (Apple Metal, 1920×1080, 150 forced frames a batch, fog and grass on, tick 90). Station: the village's west wood at the default distance and pitch (target (790, 900), 65 m, pitch 0.85, yaw −π/2), which draws 3 / 243 / 74 trees at tiers 0 / 1 / 2 and 363k triangles a view pass today, 628k now.

| Candidate | Tier 1 triangles | GPU at 65 m | At 25 m |
|---|---|---|---|
| 50 small clumps on tiers 0–1, lobed 2–3 | 4,728 | +1.61, +1.92 ms (two runs) | +1.01, +2.12 ms |
| the same, clumps merged on tier 1 | 2,248 | +0.82 ms | +0.93 ms |
| 24 clumps on tiers 0–2 (landed) | 2,432 | +0.96, +0.50 ms (two builds) | not run |

The machine was shared: two arms with identical geometry read +0.04 and +0.96 ms apart in one run, so a single number is good to about a millisecond and only the ordering is firm. The village's default camera has no forest tree in view (0 of 687), so the row is taken where the wood fills the frame. At the strategic camera every tree draws the far tier, 80 triangles before and after.

**Shimmer.** A rotation pair: the camera turned about its own eye by one pixel's angle, so the image moves one pixel with no parallax; the second frame shifted back should be the first. Mean absolute luma difference (8-bit) over leaf pixels in the middle 1000 × 600 of the frame, fog and grass off, same station.

**Pictures** (scratch, `throwaway/c73/`): `final/village-edge-65-live.png` and `-clear-`, `final/village-edge-25-clear-live.png`, `final/generated-tactical-65-live.png`, `-150-`, `final/sheet-tree_broadleaf/contact.png`; today's at the same poses in `v3/*-before.png`. The generated map is `mixed`, `medium`, seed 2, aimed at the trunk nearest its opening camera (3530, 702) at 65 m and 150 m, pitch 0.85, and 25 m, pitch 0.22. C62's stations were not on this branch; these poses are to be re-shot from them.

**Critique** (unprimed, on the final shots). No dark region read as the unseen overlay. It found nothing broken in the new geometry. Its findings, and what was done:

- Blade-thin trees at forest edges: today's trees show them at the same places. Placement scales a crown to its room inside the forest's shape with no floor on its width. Left for the forest slices (C75, C76); not this slice's file.
- Flat grey-tan patches under crowns at 25 m: gaps between clumps showing the far forest floor. Not re-shot.
- Near-black facets on the unlit side of the model sheet, faceted clump outlines, pole-like trunks under a closed lid: taste; one round only.
- Blob-shaped shadows on the forest floor and darker trees on one side of the wood: the floor's dapple and the species tint, both today's.

**Not done.** No `compare-screenshots` telemetry against `dryad.jpg` (the crowns are closed where dryad's are open, by the cost verdict, so the comparison would not have changed a decision). No village scene run: its tree checks read counts and placement, which this slice does not move, and the one that reads crowns (straight down, crowns darker than the field beside them) was checked with a single frame at the scene's own pose (124 against 167).
