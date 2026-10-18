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

## Parameter-effect map

| Parameter | Observed effect | Scope and uncertainty |
| --- | --- | --- |
| Skeleton clip set | Removes unrelated skeleton bundle downloads; correctness is unchanged because every loaded appearance's skeleton is admitted before decode | Applies to scoped battles; byte reduction depends on catalog composition. End-to-end FPS effect is indirect and unmeasured until art is fetched. |
| Static model repacking | Existing implementation already retains static prop/scenery buffers and rewrites only changed batches | No new change made; needs a valid GPU benchmark before another intervention. |

## Handoff

The incumbent is the skeleton-scoped loader. The next experiment is to fetch the
reel's LFS assets, record matched browser/native baselines, then profile the
largest GPU span before changing a shared pass.
