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

## Facade surfaces (C24 to C26)

| Owner and workload | Off | On | Evidence and scope |
|---|---:|---:|---|
| C24 cutouts: 1,728 window guards and two fence panels, default camera over a field of 288 lab blocks (1,280 models in view) | 2.07 ms | 2.32 ms | Median paired difference +0.25 ms (six rounds: −0.06 to +0.35). |
| C24 cutouts: the whole field from 420 m (4,626 models) | 2.72 ms | 2.78 ms | +0.06 ms (−0.53 to +0.32). |
| C25 glass: a pane in each of 2,592 bays, default camera | 2.13 ms | 2.29 ms | +0.16 ms (−0.17 to +0.44). |
| C25 glass: the whole field | 2.75 ms | 2.73 ms | −0.02 ms (−0.13 to +0.18). |
| C26 rooms: a room box behind each of 2,592 windows, every tier, default camera | 2.08 ms | 2.26 ms | +0.18 ms (−0.10 to +0.41). |
| C26 rooms: the whole field | 2.68 ms | 2.82 ms | +0.14 ms (−0.30 to +0.28). |
| C87 ground lane, village benchmark short run, four alternated pairs | 5.53 to 6.35 ms GPU mean; 48 to 56 FPS; 277 to 297 MiB heap | 5.66 to 6.32 ms GPU mean; 45 to 49 FPS; 426 to 454 MiB heap | Start `ca7bb972`, final `ad50cdec`. GPU pair differences +0.37, +0.13, −0.37, +0.05 ms (median +0.09) against a +3 ms lane budget. The FPS and heap changes are every lane's since the start and are not attributed; see [C87](slices/C87-ground-composition-gate.md#outcome). |

Apple metal-3, 1920 × 1080, development build, `FACADE_COST=1 scene -- facade`: each kind of surface drawn against not drawn (its depth, its shadow and its colour together), interleaved in batches of 120 forced frames, whole-frame GPU time. "Off" is "on" less the median difference. Other sessions were rendering on the same machine, so a difference under about 0.3 ms is not distinguishable from none; no kind costs more than that here. The lab's field is flat ground and one small block repeated, not a town: it bounds what the three stages cost a pixel and a draw, not a Metro map's frame. No real kit has these surfaces yet.
