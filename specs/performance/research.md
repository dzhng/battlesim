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
| Static-world walk/repack | The presentation reselects static batches when the camera changes; the renderer already retains unchanged GPU buffers | The current reel's CPU cost is dominated outside the easily removable model pack; no safe cache candidate was found without delaying authored LOD transitions |

## Handoff

The incumbent is the skeleton-scoped loader. The first clean baseline after
fetching the reel's LFS assets confirmed a repeatable `market-town` 1% low below
the agreed 50 FPS trigger. The browser is not GPU-saturated in those runs, so
the next experiment is a CPU/frame-pacing profile before changing a shared pass.

## Reactive baseline — 2026-10-19

Two production `graphics-fps` runs at 1280×720, with no other browser or native
renderer using the GPU:

| Run | Scene | Average FPS | 1% low FPS | Frame p95 (ms) | CPU p99 (ms) | GPU mean (ms) |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| `05-00-09` | market-town | 75.9 | 35.9 | 24.10 | 21.16 | 11.92 |
| `05-00-09` | paris-corner | 107.8 | 57.2 | 16.67 | 1.89 | 7.72 |
| `05-03-34` | market-town | 82.2 | 38.9 | 16.72 | 18.47 | 10.93 |
| `05-03-34` | paris-corner | 110.5 | 54.9 | 16.66 | 1.97 | 7.32 |

Evidence is in `throwaway/perf/2026-10-19/` and
`throwaway/perf/2026-10-19-repeat/`. These runs establish the trigger but do
not identify the responsible CPU span; no optimization has been selected yet.

A 20-second steady-state Chrome CPU profile during `market-town` is in the
ignored `throwaway/perf/2026-10-19/cpu-profile-market-town.json`. It sampled
mostly idle time; the active samples were split across WASM presentation
functions, GPU `writeBuffer` calls and garbage collection. The profile did not
justify a static-model cache or another renderer change by itself. The next
experiment must correlate long frame intervals with allocation and frame-loop
activity before selecting a candidate.

The `graphics-fps` scene now records browser diagnostics alongside its existing
timing contract. The first passing instrumented run
(`browser-2026-10-19T05-23-26-636Z.json`) recorded six long tasks, including a
943 ms task while `market-town` was active, and no GC entries. Its heap snapshot
was about 1.95 GB used of a 4 GB limit. Frame timestamps are recorded in the
same evidence so the next pass can locate the affected interval without adding
a second timing harness.

## Visual verification (2026-10-18)

The `frame` scene was rerun on a hardware AMD RDNA3 adapter after fetching the
approved LFS baseline. Its world render matches the baseline: terrain, tank and
vehicle silhouettes, shadows, camera framing, depth ordering, and colour remain
aligned. The scene reports 7,318 differing pixels because the candidate lab
panel has native vertical and horizontal scrollbars that are absent from the
approved image; the world crop is unchanged. This is a harness/browser UI
artifact outside the renderer changes and is retained as a follow-up rather
than hidden by blessing a new baseline.

The native parity plate was also inspected at 1280×720. The plate intentionally
includes grass while its paired plain plate suppresses grass for measurement,
so that pair is not a before/after quality gate. It confirms that the grass
pass draws dense foreground clumps and that the tank remains visible; the
native run remains directional because the benchmark adapter is llvmpipe.

The collaborative preview browser could not provide a hardware adapter and
showed only its loading/blank states (`No available adapters`), so those captures
are diagnostic and are not used as renderer acceptance evidence. The unprimed
critique found no renderer defect in the hardware scene; it did flag the panel
scrollbars as the visible UI discrepancy.
