# S4: renderer fog

**Depends on:** none (start here). **Kind:** slice.

## Question
Which renderer fog technique holds ~2 ms at city occluder counts without losing spike 02's sharp building edges?

## Contract it unlocks
Throwaway: work in a scratch worktree, merge nothing, and write the verdict to `specs/city-maps/spikes/S4.md` (numbers table + one verdict row per question + kill check).

## API seam
Paired `FOG_COST=1` runs (`web/scenes/fog.mjs`) at 3k, 9k and 20k box occluders with 48 eyes rebuilt, against today's merge loop (`fogVisibility.ts:223`), which tests every occluder per azimuth bin.

## What the human can run or see
Paired cost tables and edge crops per arm.

## Verification
- Arms:
  - today's loop;
  - a per-eye grid cull of occluders within reach;
  - angular-sector binning per eye;
  - building tops rasterised into the terrain march.
- Gates:
  - oracle agreement ≥95% of cells outside the one-cell band;
  - building edges within ~1.4 px of spike 02's;
  - fog build ≤2 ms.

  Rasterising tops is acceptable only if it meets the edge gate.
- Also measure the whole-invalidation cost on an occluder change (`setOccluders`, `fogVisibility.ts:620`), for C20.
- **Kill:** no arm fits. City fog then needs a coarser technique, which is a look decision; reslice at G0.

## Delegated to the implementer
Arm implementations. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
Everything; nothing merges.
