# SG1: tree and dressing cost

**Depends on:** none for local art trials; S0 safe limits/S1 layouts for full-extent residency arms. **Kind:** slice.

## Question
Do branch-skeleton trees and forest dressing fit the frame at forest volume (L-G5, L-G6)?

## Contract it unlocks
Throwaway: work in a scratch worktree, merge nothing, and write `specs/city-maps/spikes/SG1.md` (numbers, one verdict row per question, and the kill check). Delete the worktree after.

## API seam
- Fork `trees.py`: one skeleton broadleaf and one conifer, 4 tiers.
- Swap them in on the village with both forests at the one rule's spacing (the east wood goes from about 146 to about 355 trunks; about 710 in total).
- Add a throwaway 20k-instance fern population through the scenery layer (`frame/sceneryLayer.ts`, `scenery/lod.ts`).
- Extend the accepted local quality arm to S1's forest coverage/counts at all selected extents. Measure whole-world references/trunks versus resident near/far drawing, full overview and rapid edge-to-edge movement; village volume alone cannot prove full-map residency. Feed these limits to S0/S3/G0.

## What the human can run or see
Paired on/off benchmark short runs, and a slow-pan capture at 65 m.

## Verification
- Measure:
  - triangles per tier per tree;
  - scenery triangles per frame at the strategic, default and ground cameras (casters pay each of 4 cascades);
  - worst-window GPU p95 delta;
  - `prepare` CPU ms;
  - crown shimmer (luma variance of crown pixels over a 2 s pan, against today's).
- **Kill if:**
  - the forest adds >1.5 ms GPU at the default camera; or
  - tier 0 is >25k triangles; or
  - shimmer is >2× today's, or a critique calls it flicker; or
  - the ferns cost >1 ms CPU or >1 ms GPU.
- **Fallbacks, in order:**
  1. skeleton on tiers 0–1 only, with the lobed crown for tiers 2–3 and casters;
  2. merge leaf clumps;
  3. dressing within a radius only;
  4. dressing grown on the GPU like grass (a reslice).
- **Output:** a per-tier tree triangle budget, sent to GG **and G0** (street trees share the city frame).

## Delegated to the implementer
The throwaway plumbing. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Everything; nothing merges.

## Feedback that would change this slice
An unaffordable tree/dressing cost changes the proposed budgets/counts before production art expands.

## Verdict: trees (2026-10-01, run inside C73)

The tree half of this spike was answered on the way through [C73](C73-tree-skeleton.md); its Outcome has the numbers, the station and the method. There was no scratch worktree and no `spikes/SG1.md`: the experiment was C73's own trees, today's against the new, paired in the real lab. The dressing half (the fern population, `prepare` CPU, full-extent residency) is not run; it is C79's.

| Question | Verdict |
|---|---|
| Does the forest add at most 1.5 ms GPU at the default camera? | Yes at the budget below: +0.50 and +0.96 ms on two builds of the same counts. Open crowns of 50 small clumps did not: +1.6 to +1.9 ms. |
| Is tier 0 at most 25k triangles? | Yes: 9,312. |
| Is shimmer at most 2× today's? | Yes: 1.05×, once every drawn tier's caster has the drawn tier's shape. With the lobed crown casting for clumps it was 3.2 to 3.4×, and fallbacks 1 and 2 alone did not move it. |
| Which fallbacks were taken? | Fallback 2, merged clumps (about 24 a crown, on every clump tier). Fallback 1 was changed: clumps run to tier 2 and only tier 3 is lobed, because tier 2 casts for tier 1. |

**Kill check:** none fired for the landed trees.

**Output, for GG and G0: a tree's triangles per tier are at most 10,000 / 2,500 / 500 / 80.** The validator enforces it for every `tree` appearance, street trees included. Tier 1 is the one that matters: it is what the default camera draws, and the measured kill line for it is near 4,000.

**Not measured:** the conifer the spike asked for (C74 builds it inside this budget), both forests at 710 trunks (the village has 687 already), the worst-window p95 (a noise number on a shared machine), and a 2 s pan (one rotation pair stood in for it).
