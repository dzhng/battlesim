# SG5: distance field at selected extents

**Depends on:** S0 safe resource limits for full-extent arms. **Kind:** throwaway spike.

## Question
Which bounded structure supplies exact-enough road/forest/river distances at the selected map sizes without per-fragment full-shape loops?

## Contract it unlocks
`spikes/SG5.md` records frozen local edge probes and full-extent inputs, error/cost/memory/load tables, device limits and one verdict. Nothing merges.

## API seam
- Arm A: signed-distance channels, clamped to their useful band, with bounded tiled/resident storage where a full texture cannot fit.
- Arm B: exact segment-bucket queries with sparse/bounded storage and the shared sim distance function.
- Feed C65-style densified strokes and representative S1 town roads/forests/rivers at all fixed extents. A small village probe remains the quality oracle, not the shipping scale proof.

## What the human can run or see
25 m edge-error heat maps and paired GPU/load/residency tables at Small/Medium/Large, including whole-map overview and rapid pan.

## Verification
- Edge error ≤0.25 px at 25 m, including joins and variable widths; terrain/grass cost compared with today's loops.
- Logical coverage versus resident/peak bytes, empty-page cost, device texture/buffer limits, build/upload time and edge-to-edge updates.
- An arm exceeding S0's safe experiment limit is rejected analytically before allocation. Dense 1 m fields cannot be assumed affordable merely because they fit at 1.6 km.
- Kill an arm if it misses quality or G0/GG memory/load/GPU budgets, gives no GPU win, or cannot cover the fixed extents. Reslice the field owner; do not shrink maps or introduce a renderer-only physical rule.
- Compare error/edge crops with exact geometry via compare-screenshots; run unprimed screenshot-critique last; preview-shots non-blocking.

## Delegated to the implementer
Experimental packing/index arms and instrumentation. GG selects the structure consistent with G0's full-extent architecture.

## Must stay green
Signed distances, sim agreement and one field owner; nothing merges.

## Feedback that would change this slice
An error or residency miss reopens that structure and C63's production contract.
