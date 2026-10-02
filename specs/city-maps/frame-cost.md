# City renderer frame cost

These measurements admit their named workload only. Full Metro and G0 admission still require the frozen extent, density and material inputs selected by the wider city gates.

| Owner and workload | Before | After | Evidence and scope |
|---|---:|---:|---|
| C20 fog rebuild: 16,000 synthetic boxes, 48 separated eyes, 600 m reach | 37.831 ms | 1.182 ms | ≤2 ms local gate passes. Existing horizon intersections retained; 2,310,144 ray/box visits versus 3,145,728,000 global. |
| C20 frozen village fog rebuild: 3 boxes, 9 eyes | 1.346 ms | 0.999 ms | No observed regression; GPU telemetry varies with this adapter's load and clocks. |
| C20 stable synthetic-city fog cull | 0.0040 ms | 0.0032 ms | Maps and whole flags already built; this is not the rebuild cost. |
| C20 synthetic-city GPU allocations | 57,203,152 buffer bytes; 1,927,196 texture bytes | 57,506,980 buffer bytes; 1,927,196 texture bytes | +303,828 buffer bytes for the local candidate tables; 20 →21 buffers, four textures unchanged. |

Apple metal-3, Chromium 148.0.7778.96. Frozen baseline `4efec25b`; candidate `f8251824`. Compute-pass timestamp spans cover terrain, merge, whole-surface and cull, with paired off/on/rebuild rounds after three warmups; each value is the median of twelve rounds. Render-only markers were found not to enclose Metal's compute work and are excluded. The grid-only intermediate measured 2.611 ms and was rejected against the 2 ms gate.

Raw samples, flags, vectors and the frozen village input are in ignored scratch `throwaway/fog-paired-cost.json`; the intermediate is `throwaway/fog-grid-cost.json`, and the final runner/log are `throwaway/fog-sector-cost.mjs` and `throwaway/fog-sector-cost.log`. The work census and its reproducer are `throwaway/fog-candidates.json` and `throwaway/fogCandidateProbe.test.ts`. This synthetic separated-region test fulfills the startup lane's 16,000-building work-count question; it does not establish actual dense Metro's complete frame cost or the missing S4/G0 input verdict.

C20's matched browser look and limitations are recorded in [its outcome](slices/C20-renderer-fog-at-scale.md#outcome). Full capture metrics, crops and unprimed findings are in ignored scratch `throwaway/fog-comparison/`.
