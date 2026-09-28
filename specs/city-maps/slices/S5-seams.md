# S5: seams

**Depends on:** S2's driver. **Kind:** slice.

## Question
Can joins be hidden where a building's footprint parts meet, and between terrace neighbours (party walls)?

## Contract it unlocks
Throwaway: work in a scratch worktree, merge nothing, and write the verdict to `specs/city-maps/spikes/S5.md` (numbers table + one verdict row per question + kill check).

## API seam
Bake a real L-shaped 3-part footprint and a 5-house terrace from S1's crop, with neutral materials.

## What the human can run or see
Tight facade and roof-edge crops at 30, 80 and 250 m, in Blender and in S3's harness (which catches z-fighting from forced two-sided faces, L11).

## Verification
- Arms:
  - parts abutting;
  - parts overlapping by 0.3 m;
  - a patched **exposed-edge input** that suppresses facade, piers, cornice ends and windows on flagged edges.
- Unprimed critique question: "do you see where one building or part meets another?"
- **Kill → fallback:** constrain decomposition to rectangles aligned to one frame, and mask joins with a pilaster module.
- **Decides for G0:** the seam strategy C13 bakes, and the exposed-edge data C04 must emit.

## Delegated to the implementer
Graph patch mechanics. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Everything; nothing merges.
