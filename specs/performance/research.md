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

The active target is now the production Metro XL stress case, at 100 units per
side. Earlier market-town frame pacing was accepted by the user at roughly
39 FPS 1% low. The XL findings below supersede the earlier CPU-first priority.
The controlled distant-car change saves about 12% GPU time; this does not prove
the original 25% browser/native frame-time target. Native hardware performance
and the remaining XL frame-time gap are still open.

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
was about 1.95 GB used of a 4 GB limit. The full diagnostic run matched a
997 ms `market-town` long task to a 1,050 ms frame interval. Chromium reported
the task as an unattributed window task, so the next pass needs targeted
renderer or frame-loop counters to name its owner. Boundary spans then showed
a 1,443 ms `presentation-frame-build` inside a 1,565 ms long task, while model
preparation was about 0.9 ms and renderer draw about 5 ms. Static selection was
only about 6.5 ms in the same window, so the expensive portion is the loaded
unit/effect presentation work. A Rust timing hook was attempted but removed
because it changed the reel's runtime; the JS boundary remains the accepted
measurement. Further probes that wrapped the presentation boundary changed the
long reel runtime and were removed; the accepted evidence remains the earlier
successful JS boundary run.

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

## Rust scratch reuse measurement (inconclusive) — 2026-10-19

The first output-equivalent Rust pass reuses `Loaded` scratch vectors for
high-air shadows and x-ray membership lists. The focused Rust presentation
suite and commit hooks passed. A browser `graphics-fps` run reached the
`market-town` stage but did not complete within five minutes and was stopped;
there is no accepted before/after FPS result. The pass therefore remains an
allocation-hygiene candidate, not a measured trigger improvement. The next
measurement must use a bounded, completing reel before ranking another
intervention.

## Native A/B for scratch reuse — 2026-10-19

The existing native replay bench ran the same captured `paris-corner` workload
for 2,400 frames against the immediate pre-pass source and the committed pass.
The pass reduced total CPU p50 from 1.31 ms to 1.14 ms and p95 from 1.62 ms to
1.48 ms. GPU mean was 8.05 ms before and 7.99 ms after, within normal run
noise. Counts and the drawn workload were identical. This supports retaining
the output-equivalent pass, but it does not replace the browser trigger check:
the browser reel still needs one bounded completing run.

The graphics scene now has a two-minute scene-local deadline for reaching its
`results` state. A stalled page therefore fails with a direct diagnostic
instead of waiting on the harness's much longer outer timeout; this does not
change any successful-run timing contract.

The bounded validation reproduced the stall on `FPS_SCENES=paris-corner` and
failed at 120 seconds with `graphics test did not reach results`. That isolates
the current blocker to the browser graphics-test completion path rather than a
map-specific workload; no post-pass FPS claim is valid yet.

The runner selects local Playwright Chromium on this macOS worktree and passes
`--enable-unsafe-webgpu --enable-features=WebGPU`; it is not falling back to a
second map or an unconfigured browser. The page still produces no first frame
or adapter identity before the watchdog, so the remaining diagnosis belongs to
the local headless WebGPU/resource path rather than Rust frame generation.

The route now mirrors the runner deadline in its own state: a stalled running
reel changes to the existing incomplete-test results view after two minutes.
This does not make the missing timing evidence valid; it makes the failure
visible and machine-readable instead of leaving a blank running page.

## Browser reel recovered — 2026-10-19

The initial completion failure was caused by stale ignored WASM artifacts in the
worktree. Rebuilding `web/src/wasm` with `bun run --cwd web build:wasm` removed
the teardown ownership error; the `frame` scene then passed its hardware adapter,
interaction, and GPU resource checks. The final two-map graphics reel completed:
`market-town` averaged 83.1 FPS with a 39.0 FPS 1% low and 10.73 ms GPU mean;
`paris-corner` averaged 110.5 FPS with a 58.8 FPS 1% low and 7.44 ms GPU mean.
The tracked source did not need a lifecycle workaround; generated WASM remains a
local build artifact and must be rebuilt after source or branch changes.

## Rejected transit scratch reuse — 2026-10-19

An output-equivalent experiment reused `TransitHistory::tracks`' result vector
inside `Loaded`. The native `paris-corner` bench did not support it: before was
1.12 ms CPU p50 / 1.31 ms p95, after was 1.16 ms / 1.40 ms, with unchanged
drawn counts and GPU means. The experiment was reverted; transit reconstruction
is not a selected optimization target.


## Building-walk list reuse — 2026-10-19

`BuildingWalk::select` rebuilt every module/tier list and its count vector on
each camera walk. A two-buffer implementation keeps the prior lists for exact
comparison and reuses the other buffer's capacities for the next walk; counts
are cleared and reused. This bounds the normal per-walk allocation churn without
changing list order or records.

The immediate native control measured 1.14 ms CPU p50 / 1.45 ms p95. Two reuse
runs measured 1.06 / 1.30 and 1.05 / 1.52 ms, with identical drawn counts and
GPU means around 7.95 ms. The focused presentation suite passed. Bounded browser
reels also completed: `paris-corner` reached 111.4 FPS average and 57.6 FPS 1%
low; `market-town` reached 83.2 FPS average and 39.1 FPS 1% low. The latter
remains below the research floor, but the user accepts this result for the current
player-facing target. The pass is retained as a measured CPU improvement, and no
further optimization is selected without a new symptom.

## XL 5v5-scale stress workload — 2026-10-19

The endurance scene's generated arm loads the full Metro XL world (10 × 10 km)
with 100 active units per side, matching the planned 5v5 force size. The run
uses the production view and samples frame intervals in ten-second windows.
With the complete roster's realistic remains state—800 fallen soldiers and 100
wrecks—the 30-second sample measured:

- live contact: 20.4 simulation Hz, 50.0 ms frame median and 66.6 ms worst p95;
- late state: 15.9 simulation Hz, 58.2 ms frame median and 74.9 ms worst p95.

The generated world contains 12,123 buildings and 199,891 props; GPU buffers
were about 3.08 GiB and full page memory about 4.4 GiB. This workload is kept as
a diagnostic scale scenario rather than folded into the 60-FPS graphics reel
gate. The scene's stale Large-versus-XL assertion was corrected while recording
this evidence.

## XL render-scale experiment — 2026-10-19

The XL stress workload is GPU-bound in the endurance runner’s default 1280×800 CSS viewport: the minimum-quality run measured
about 41.2 ms GPU mean live and 48.8 ms late. The frame uses several full-size
4× multisampled targets; a 1× experiment was rejected by WebGPU because the
renderer’s depth bindings and shader contracts require multisampled depth.

A bounded render-scale profile was then tested at 0.75× for the internal canvas
only (the HUD remains at CSS resolution). The same generated map, roster, late
state, and deterministic digest measured 35.8 ms GPU mean live and 45.7 ms late,
with GPU texture memory dropping from about 610 MiB to 548 MiB. This is roughly
13% live and 6% late GPU improvement, with no simulation or presentation-output
change. The scale is used by the production Low profile and the test-only minimum
profile; Medium and High remain at 1×. The low-quality graphics reel completed
without render errors.


## Corrected XL GPU attribution — 2026-10-20

The previous 27 ms “backbone” and short optional-layer toggles are invalid
attribution: GPU statistics averaged 240 frames while toggles waited only
1.8 seconds, mixing arms. Later production runs using `test-quality=low` did
not select Low after the development-only override was guarded. Do not use
those runs to assign CPU/GPU percentages or compare resolutions. CPU/GPU time
can overlap; different-run GPU means and frame medians are not an additive
per-frame breakdown.

The corrected probe persists the real Low preference before loading production,
confirms the 960×600 target at 1280×800 CSS, pauses the generated Metro XL seed-4
opening with 100 units per side, reduces the diagnostic GPU window to 30
samples, and waits 90 newly drawn frames after every toggle. Full baselines
interleave each omission. On the same adapter:

| Omitted work | GPU mean | Interpretation |
| --- | --- | --- |
| Nothing | about 32.2 ms | Paused opening baseline |
| Terrain shadow draws | about 31.9 ms | About 0.4 ms; low priority |
| All shadow encoding | 22.4–23.5 ms | About 9 ms total marginal cost |
| Model-layer shadows | 25.33 ms | About 6–7 ms versus adjacent baselines |
| Building/scenery shadows | about 31 ms | Near the baseline noise floor |

These omissions diagnose cost; they are not output-preserving candidates.
Deltas are not additive on a tile GPU. The sampled opening has only two sparse
scar pages, so this does not characterize a large late-state scar footprint.
Scratch evidence is in `throwaway/xl-audit/` (not versioned).

Full-map terrain submission remains structurally unbounded (643,044 vertices in
this world), but it does not explain most of the measured shadow overhead.
Prioritize conservative model/prop caster filtering against actual fitted
cascade volumes. Configured shadow reach alone is insufficient because cascade
fitting can extend farther. A fixed-view map-size invariant requires matching
local visible content and a finite view distance; an infinite-horizon camera
may legitimately admit more geometry on a larger map.

## XL candidate ledger — 2026-10-20

All comparisons use the settled protocol above. Means below are milliseconds;
A is the adjacent/full baseline, B the candidate or diagnostic omission. Omission
probes identify marginal cost, not shippable changes. Scratch code, logs and
screenshots remain under `throwaway/xl-audit/` and are not product hooks.

| Experiment | A | B | Decision |
| --- | --- | --- | --- |
| Prop casters filtered against cascade union | 30.93 / 30.89 / 30.19 | 30.32 / 32.03 / 32.27 | Reject extra lists/storage; no repeatable gain |
| Omit prop shadows | 32.26 / 32.54 | 25.54 | Props dominate model-shadow cost |
| Omit unit/remains shadows | 32.54 / 32.09 | 32.08 | Near noise in this opening, not a late-state verdict |
| Single-joint vertex fast path | 31.60 / 32.42 / 31.70 | 31.67 / 30.57 / 31.82 | Reject; no consistent benefit |
| Separate prop list per cascade | 31.72 / 31.71 / 32.15 | 32.04 / 31.84 / 31.55 | Reject expanded batch contract/storage |
| Omit solid model shadows | 30.37 / 30.18 | 21.85 | Solid geometry dominates |
| Omit cutout model shadows | 30.18 / 30.19 | 30.23 | Near noise |
| Equivalent cutout discard | 30.19 / 30.31 | 30.16 / 30.23 | Reject as a performance candidate |
| Position-only solid depth vertex stage | about 30.2 | about 30.2 | Reject additional shader ownership |

The solid/cutout and depth-stage probes opened the pause-menu overlay in both
arms; use their paired deltas, not absolute comparisons to other runs. None of
the renderer experiments is retained. Filtering was conservative, but selected
work alone did not establish a measurable GPU saving.

## Dominant prop appearances

The opening submits about 100,000 visible props and 13.5 million model triangles
at the coarsest tier. Parked cars dominate: 23,991 × 300 = 7,197,300 triangles.
Courtyard walls contribute 43,386 × 88 = 3,817,968; planters contribute 481,936.
Paired shadow omission puts parked cars around 3 ms marginal cost; wall and
planter deltas are smaller and noisier. The next practical owner is authored
distant meshes. Non-impostor props continue drawing their coarsest mesh at any
distance, so unnecessary detail multiplies across large visible populations.

The parked-car candidate removes buried window slab faces, subpixel wheel-arch
clearance and small bumper geometry only from its distant tier: 172 triangles
versus 300. Exported mesh attributes, indices and material indices for tiers
0–2 match exactly. The source excludes wreck generation from all changes.
The loader’s existing per-tier budget mechanism bounds future distant complexity.

The full catalog bake passes. Matched car sheets exercise the changed tier
(the final frame reports 172 triangles in tier 3); their image differences are
minute and an unprimed visual review found no visible regression. Tier-2 cars
also cast using tier 3, so the mid view’s shadow comparison matters. The corrected production XL comparison switches both baked meshes in one paused
battle and asserts equal instance counts (100,370) and exactly 3,070,848 fewer
triangles in each candidate arm. Baseline GPU means are 32.57 / 32.81 / 31.94 ms;
candidate means are 27.44 / 29.58 / 28.63 ms: about 3.9 ms (12%) lower on average.
No cars are omitted. Live/late-state confirmation and closeout checks remain
pending. This is GPU duration, not a claim about achieved frame cadence.

## Direct culling audit

To distinguish selected from actually in-frustum geometry, a throwaway probe
copied each held prop record and transformed all eight corners of its authored
state bounds (including instance scale and yaw) through the actual GPU camera
matrix. It tested all six WebGPU clip inequalities, independently of Rust’s
sphere/chunk selection. The original assets and production Low were restored
for this audit.

Of 100,340 selected props, 99,580 bounds lie wholly inside, 315 intersect a
boundary, and 445 lie wholly outside near the screen edges. No records or
appearances were missing. Thus 99.6% intersect the view: frustum culling is
working for this sample. The selected world positions span roughly x=2882–6695,
y=4035–6801 within the 10 km square map. This camera is 2400 m from its target,
with pitch 0.95, vertical FOV 0.8 and aspect 1.6.

There are 23,991 parked cars; their projected bounding boxes span only 0.8–2.2
rendered pixels. Coarsest-mesh complexity therefore remains relevant even with
correct frustum culling. This is not an occlusion audit: geometry hidden behind
other geometry still submits vertices before GPU depth rejection.

Per-cascade bounds exclude 12,403 / 3,205 / 216 / 0 props respectively: only
about 4% of total repeated prop submissions. The union excludes none. This
explains why the earlier conservative selection candidates did not help; their
failure alone was not evidence that culling was correct. Full terrain submission
remains a separate unbounded path, with small measured shadow cost here.

The first old/new car-mesh timing comparison is invalid: the temporary baseline
alias was excluded by the application’s appearance-selection filter, so that arm
omitted parked cars. No timing claim is accepted from it. A valid comparison must
assert equal instance counts and the expected different triangle counts first.

### Production confirmation of the distant-car change

The generated Metro XL seed4 endurance run on production Low passed 30 seconds
of live contact and 30 seconds of late-state contact. GPU frame mean/p95 was
31.9/35.3 ms live and 34.1/36.3 ms late. Worst ten-second-window frame p95/p99
was 58.3/75.0 ms live and 99.9/209.2 ms late. These absolute results are not a
paired speedup estimate; the controlled opening A/B above owns that claim.

Three resets reproduced digest `60ede87ba60aab0b`, camera and corpse identities,
and identical GPU allocation counts and bytes. Both contact arms involved
50 own units firing. Evidence: `throwaway/xl-audit/car-production-endurance.log`
and `car-production-telemetry.json`. The run confirms production integration
and reset stability, not achievement of the whole performance target.

### Distant-wall simplification rejected

A three-band courtyard-wall mesh reduced the far tier from 88 to 40 triangles
and preserved the near tiers' exported mesh bytes. Matched source preview
sheets and an unprimed review showed less contrast and a less distinct upright
wall at the farthest view. It was rejected before GPU benchmarking; no speedup
is claimed. Scratch evidence: `throwaway/xl-audit/wall-source-before`,
`wall-after`, and the paired `wall-*-distant.png` crops. Both raw-source previews
lack catalog-kind metadata, so they are image comparisons, not asset-validation
proof. The original wall remains in production.

### Native consumer confirmation

After hydrating the newly committed car bundle from the local LFS cache,
`cargo test -p native-client --test props` passes both placement/appearance
checks. `cargo test -p native-client --test counts` also passes: the committed
browser replay's frame counts match native at tick boundaries and between.
These prove shared asset consumption and presentation-count parity, not native
GPU speedup. Logs: `throwaway/xl-audit/car-native-props-hydrated.log` and
`car-native-counts.log`.

### Remaining terrain work bound

Both clients currently submit the complete terrain triangle list to depth,
colour and each shadow cascade. A future correction should partition immutable
triangles into spatial draw ranges once, then select them in Rust presentation
for the camera and each actual cascade matrix. Browser and native must consume
the same ranges. Selection itself must use a spatial query rather than scan all
chunks, and partitioning must preserve vertex positions, normals and tint.

The current view projection intentionally has no far plane. A horizon-facing
view can therefore admit more local content as a map grows. Strict map-size
independence at every pitch needs an explicit finite visibility/fade policy;
chunk selection alone cannot promise it. A downward fixed-view comparison can
hold local content constant without changing that policy.

Before adding this cross-client contract, measure the whole-terrain omission
ceiling on the production Low XL opening. The earlier terrain-shadow omission
saved only about 0.4 ms. Omission changes the picture and is never an acceptable
shipping optimization; its result only ranks the possible work. Required gates
for a real chunk implementation are equal selected triangles when distant map
content grows, retention of crossing triangles and off-camera casters affecting
visible ground, bounded selection visits, browser/native range parity, and
matched image/GPU comparisons.

The post-car omission probe completed at 960×600, paused tick35, with the
same 100,370 model instances and 10,443,810 model triangles in every arm.
Interleaved normal GPU means were 25.29, 28.19, 23.20, 26.69 ms; omitted means
were 23.01, 19.43, 22.08 ms (group means 25.84 and 21.51 ms). Variation is large.
This is a rough total-terrain cost indication, not a predicted 4.33 ms culling
gain: the omission also removes visible shading and changes depth/occlusion.
The actual selection candidate must preserve those pixels. All omission hooks
and the shortened timing window were removed. Evidence is under
`throwaway/xl-audit/terrain-ceiling/` (`results.json`, `run.log`, saved probes).

### Shared spatial terrain selection rejected

A tested Rust spatial hierarchy grouped exact terrain triangles, selected ranges
using actual camera and cascade matrices, and fed both clients. Nine focused
geometry/mesh tests and native prop/count tests passed; browser typechecking
passed. Deliberately disabling culling or using centroid-only bounds failed the
regressions. The implementation added roughly 350 production lines plus 152 test
lines and introduced persistent spatial resources in both clients.

The actual production Low XL A/B alternated full and selected draws over the same
partitioned geometry. Full GPU means: 29.47, 29.70, 29.12, 29.64 ms. Selected:
29.54, 29.15, 29.10 ms. Group means were 29.48 versus 29.26 ms, a 0.22 ms delta
within observed variation. Selected terrain had 24 view vertices across three
ranges, plus 24/42/48/48 cascade vertices; five queries tested 283 hierarchy nodes.
Selection ran in both arms, so this does not establish a CPU improvement over
the original renderer. The tiny GPU difference does not justify the added
ownership, allocation and cross-client maintenance burden for the current limit.

The complete candidate and temporary hooks were removed. Scratch evidence and
source are preserved under `throwaway/xl-audit/terrain-selection/`. Full terrain
submission remains structurally map-wide; this experiment rejects its priority
as the current stress fix, not the value of spatial terrain selection at every
possible map density. Whole-terrain omission includes visible shading, so its
~4.3 ms difference was not available through off-view culling.
