# S3: full-extent frame, residency and startup

**Depends on:** S0's bounded resource arm, S1 layouts and S2 template counts (synthetic modules may start earlier). **Kind:** throwaway spike.

## Question
Can resident template chunks, terrain and far representations hold the frame and first usable view at the actual selected extents?

## Contract it unlocks
`spikes/S3.md`: frozen browser harness/inputs, frame/startup/memory tables, contact sheets and G0 residency/tier verdicts. No production code merges.

## API seam
S1 full-extent maps with S2's shared modules/local template placements. Keep compact building references for the whole world, then expand module instances only for bounded resident chunks; do not materialize every module transform on the map. Compare initial chunk/residency sizes and cheap far massing, including the full Large overview and transitions between opposite edges.

## What the human can run or see
Startup/frame tables and matched shots at ground, default, mid, tactical and whole-map overview distances. The old 2,000 m maximum alone does not frame an 18 km map. Size-aware framing is part of the proof.

## Verification
- Arms: interiors off/LOD0/LOD0–1/every tier; far-only versus detailed shadow casters; legal kit texture edges ≤1K; residency and far/very-far massing strategies.
- Measure total GPU, CPU prepare, resident/world reference bytes, instance expansion/upload, drawn triangles/calls, JS/wasm/GPU peak memory, cold load and first usable frame.
- Fast pan/zoom/orbit, edge-to-edge relocation and map replacement stay inside fixed pools; no full-map module expansion or unbounded chunk churn.
- Include dense Large Metro, dispersed Open and forest-heavy layouts. Heavy matched benchmarks use G0's representative worst cases; cheap layout validation covers the wider seed matrix.
- **Kill:** static city above 15 ms GPU or no feasible full-extent startup/residency proof. Fallbacks: reduce interior tiers, use coarse casters, shrink hero residency, simplify/merge far representation. Preserve map dimensions/type character; reslice failed owners at G0 if needed.
- Decide per-tier residency/triangle/instance budgets, overview representation, camera framing ranges, interior tiers, kit texture edge and startup/download limits.
- Visual variable: massing/residency/framing only. Compare contact sheets against S1 overlays using compare-screenshots, run unprimed screenshot-critique last, and preview-shots non-blocking.

## Delegated to the implementer
Throwaway harness plumbing and measured arms. Production resource architecture and numeric budgets are G0 outputs.

## Must stay green
Production code and fixed extents; nothing merges.

## Feedback that would change this slice
A rejected overview or transition changes the corresponding far/residency proposal and evidence.
