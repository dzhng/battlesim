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
