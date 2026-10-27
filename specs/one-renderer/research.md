# One-renderer auto-research record

## Contract

Objective: implement the one-renderer architecture and decide whether the
shared Rust `wgpu` runtime can replace the browser TypeGPU renderer without
breaking production visuals or lifecycle behavior.

Fixed comparison: the same frozen presentation frame, canvas size, adapter,
render scale, and authored runtime assets. The browser TypeGPU renderer is the
control; the browser and native shells call the shared Rust renderer candidate.

Acceptance is architectural and behavioral, not a microbenchmark win. The
candidate must preserve battle digests/publication contracts, match the
production frame within the existing screenshot gates, and pass resize, rebuild,
surface/device-loss recovery. Complexity is measured after deleting duplicate
TypeGPU owners; a small performance regression is acceptable when the total
renderer ownership and source complexity are lower.

Cheap evaluator: focused Rust/WASM checks, then the `one-renderer` browser
scene with interleaved repeats and screenshot/accounting output. Full native
parity and repository closeout run only after all production passes are wired.

## Hypothesis queue

1. **Shared resource owners first (active):** moving native model/material,
   shadow, and fog target ownership into `renderer-core` gives both shells one
   lifetime owner without changing GPU work. Falsifier: browser-safe types cannot
   represent the existing assets or native behavior without a second graph.
2. **Pass seams before browser integration:** extracting fog, effects, post and
   overlays as input traits/records will make the native frame callable from the
   browser shell while retaining pass order. Falsifier: the seam requires DOM,
   filesystem, or platform-specific resource types in the core.
3. **Lifecycle is the complexity gate:** shared resize/rebuild/reset/device-loss
   ownership removes more duplicated state than it adds. Falsifier: either
   shell needs an independent registry or cannot return to a clean baseline.
4. **Deletion is the final win:** after parity, deleting TypeGPU runtime owners
   produces the complexity reduction. Falsifier: keeping TypeGPU as a runtime
   materially lowers maintenance without duplicating rendering ownership.

## Parameter-effect map

| Strategy | Current evidence | Effect | Status |
|---|---|---|---|
| Standalone Rust production-shaped graph | Full frame distance 47.03; incomplete materials/shadows/fog/post | Fast diagnostic, invalid migration comparison | Reject as final architecture |
| Shared target/pipeline/depth/world owners | Native/browser compile; unchanged pass order/GPU work | Reduces duplicate allocation and attachment policy | Keep |
| Shared indexed model upload owner | Native/browser compile; queue upload seam | Establishes one model GPU lifetime owner; no measured extra draws | Keep, production integration pending |
| Shared fog-mask owner | Native/browser-safe core; same 2 optional distance passes + compose + rim | Removes native attachment/pass duplication with zero extra GPU work | Keep |
| Browser TypeGPU as permanent second renderer | Two pass/resource graphs | Preserves a known control at the cost of permanent duplication | Reject after parity |

## Attempt ledger

| Candidate | Parent | Result | Evidence |
|---|---|---|---|
| `a27ca5d27` shared targets/pipelines/depth/world/browser shell | native client before extraction | Compile and browser scene pass; parity incomplete | PR #16, one-renderer evidence |
| `91102bb19` + `e29e253bf` shared indexed model owner/queue seam | `a27ca5d27` | Focused native/browser checks pass; browser still uses compatibility draw path | PR #16 |
| `88155dc93` shared fog-mask resources/pass order | model-owner incumbent | Native adapter and renderer-core WASM/native checks pass; GPU work unchanged | PR #16 |

## Handoff

Incumbent: `a27ca5d27` plus the model-owner commits on PR #16.
Active task: extract fog compute/material ownership, then effects/post/overlay
and lifecycle seams. Regression set: renderer-core/native-client WASM checks,
presentation tests, and the interleaved one-renderer scene. Next test: run the
first focused fog/resource extraction checks, then rerun the browser scene only
when the shared frame path changes.
