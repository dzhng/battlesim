# Renderer performance research

## Contract

The target is a 25% relative frame-time reduction on the browser and native
renderers with unchanged presentation output. Browser and native must continue
to consume the same presentation records and asset identities. Download work is
measured as compressed runtime bytes before the first frame.

The fixed frame-rate evaluator is `web/scenes/graphics-fps.mjs` at 1280×720,
with native checked by `native-client --bench` on the same capture. A complete
run requires the runtime Git LFS art; the checkout used for this pass contains
LFS pointers for the reel bundles, so FPS confirmation is pending.

## Attempt ledger

| Candidate | Parent | Result | Evidence |
| --- | --- | --- | --- |
| Load only skeleton clips referenced by the selected roster/factions | catalog loader before this change | keep | `loader.test.ts`, `appearanceCatalog.test.ts`; typecheck passes |
| Benchmark browser/native reel | unchanged checkout | blocked | graphics test refused the run on a Git LFS pointer |
| Precompute grass clump yaw in the build pass | unchanged renderer | keep provisionally | Browser matched reel; native short replay; shader export and visual-count checks pass |
| Replace camera-record string serialization with a generation counter | grass candidate | discard | Browser CPU medians changed only 0.09 ms (market-town) and 0.01 ms (paris-corner); GPU unchanged |

## Parameter-effect map

| Parameter | Observed effect | Scope and uncertainty |
| --- | --- | --- |
| Skeleton clip set | Removes unrelated skeleton bundle downloads; correctness is unchanged because every loaded appearance's skeleton is admitted before decode | Applies to scoped battles; byte reduction depends on catalog composition. End-to-end FPS effect is indirect and unmeasured until art is fetched. |
| Grass clump yaw | Browser market-town GPU mean 15.04 ms → 14.38 ms (−4.4%); paris-corner 10.82 ms → 10.82 ms. Native llvmpipe short replay was 1389.0 ms → 1389.97 ms (noise floor) | Reuses each clump's full-precision sine/cosine in every vertex instead of evaluating both trigonometric functions per vertex. Browser win is provisional until a repeat; native result is software-adapter evidence only. |
| Static model repacking | Existing implementation already retains static prop/scenery buffers and rewrites only changed batches | No new change made; needs a valid GPU benchmark before another intervention. |

## Handoff

The incumbent is the skeleton-scoped loader. The next experiment is to fetch the
reel's LFS assets, record matched browser/native baselines, then profile the
largest GPU span before changing a shared pass.
