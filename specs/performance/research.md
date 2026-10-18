# Renderer performance research

## Contract

The target is a 25% relative frame-time reduction on the browser and native
renderers with unchanged presentation output. Browser and native must continue
to consume the same presentation records and asset identities. Download work is
measured as compressed runtime bytes before the first frame.

The fixed frame-rate evaluator is `web/scenes/graphics-fps.mjs` at 1280×720,
with native checked by `native-client --bench` on the same capture. Runtime art
was fetched for the current runs; native measurements use the available
llvmpipe adapter and are therefore directional for hardware GPU performance.

## Attempt ledger

| Candidate | Parent | Result | Evidence |
| --- | --- | --- | --- |
| Load only skeleton clips referenced by the selected roster/factions | catalog loader before this change | keep | `loader.test.ts`, `appearanceCatalog.test.ts`; typecheck passes |
| Benchmark browser/native reel | unchanged checkout | blocked | graphics test refused the run on a Git LFS pointer |
| Precompute grass clump yaw in the build pass | unchanged renderer | keep provisionally | Browser matched reel; native short replay; shader export and visual-count checks pass |
| Replace camera-record string serialization with a generation counter | grass candidate | discard | Browser CPU medians changed only 0.09 ms (market-town) and 0.01 ms (paris-corner); GPU unchanged |
| Carry model yaw sine/cosine in the shared instance record | grass candidate | invalid | The graphics reel exceeded its 900 s scene timeout after the record-layout expansion; reverted before promotion |
| Reuse the existing model colour word for packed yaw sine/cosine | unchanged renderer | discard | Full browser reel completed, but market-town GPU 14.41 → 14.48 ms and paris-corner 10.79 → 10.92 ms; reverted |
| Skip shadow PCF taps outside a cascade | unchanged renderer | keep provisionally | Browser market-town 14.41 → 14.36 ms and paris-corner 10.79 → 10.82 ms; native llvmpipe 1388.71 → 1387.03 ms. Output is identical outside the cascade; gains are near the measurement noise floor |
| Unroll the fixed five-tap Vogel PCF kernel | shadow early-return candidate | keep provisionally | Browser market-town 14.36 → 14.35 ms and paris-corner unchanged; full reel and pipeline validation pass |
| Specialize fixed shore-noise rotation angles | shadow PCF candidate | discard | Full browser reel completed, but market-town GPU 14.35 → 14.38 ms and paris-corner 10.82 → 10.83 ms; reverted |

## Parameter-effect map

| Parameter | Observed effect | Scope and uncertainty |
| --- | --- | --- |
| Skeleton clip set | Removes unrelated skeleton bundle downloads; correctness is unchanged because every loaded appearance's skeleton is admitted before decode | Applies to scoped battles; byte reduction depends on catalog composition. End-to-end FPS effect is indirect and unmeasured until art is fetched. |
| Grass clump yaw | Browser market-town GPU mean 15.04 ms → 14.41 ms (−4.2%); paris-corner 10.82 ms → 10.79 ms. Native llvmpipe short replay was 1388.71 ms → 1389.97 ms (noise floor) | Reuses each clump's full-precision sine/cosine in every vertex instead of evaluating both trigonometric functions per vertex. Browser win is provisional until a repeat; native result is software-adapter evidence only. |
| Static model repacking | Existing implementation already retains static prop/scenery buffers and rewrites only changed batches | No new change made; needs a valid GPU benchmark before another intervention. |
| Shadow receiver sampling | The shader now returns unshadowed before PCF when projection is outside the cascade | Applies to all clients and preserves the existing cascade contract; hardware-scale gain remains uncertain |
| PCF kernel setup | Fixed radii/directions are constants; one per-fragment rotation replaces five per-tap angle calculations | Applies to all clients; measured gain is currently near noise floor |

## Handoff

The incumbent is the skeleton-scoped loader. The next experiment is to fetch the
reel's LFS assets, record matched browser/native baselines, then profile the
largest GPU span before changing a shared pass.
