# S0: full-extent allocation and startup

**Depends on:** none (start here). **Kind:** throwaway spike.

## Question
Can the fixed battlefield extents exist and boot within a bounded native/wasm/browser memory and startup budget?

## Contract it unlocks
A frozen experiment under `throwaway/city-spike/` and `spikes/S0.md`: allocation model, inputs/hashes, safe current-path measurements and a pass/fail inventory per storage/export owner. No production code merges. The fixed extents are defined in [the completed map](../procedural-maps.md); do not substitute a smaller battlefield.

## API seam
Today's `MapDefinition` → terrain, navigation, foliage/occlusion, world exports, wasm memory and browser/GPU upload. Inventory permanent and peak temporary allocations and transfer copies before allocating. A dense allocation model may reject an arm without executing it. Establish a safe experiment memory ceiling from this Mac's available capacity and document it.

Use a cheap flat full-extent layout at each selected size, then sparse occupied regions at opposite edges. Do not build alternative terrain/nav/export architectures in S0. A predicted ceiling breach is a valid failed arm. G0 can immediately use that verdict to add one focused representation proof per failed owner; dependent measurements remain BLOCKED until its proof succeeds. All consumers must query the same physical surface; local camera windows cannot remove distant simulation activity.

## What the human can run or see
An allocation table and native/browser cold-start probe showing exact bounds, peak memory, export/upload copies, first usable frame and activity at both edges. Explain which failures were predicted versus executed.

## Verification
- Empty and sparsely occupied maps at Small, Medium and Large; resolutions varied independently of extent.
- Measure world/nav/fog/foliage storage, lazy navigation scratch/clearance, terrain mesh/export, wasm growth, worker/browser copies and GPU upload/residency. Name `useStaticWorld.ts`'s main-thread `WorldView` beside the worker Battle: both currently build world geometry. Count their allocations and exported/transfer copies separately.
- First route, first publication, resubscription and opposite-edge activity expose lazy/full-map allocations.
- Freeze inputs, toolchain/dependency versions, output hashes and evidence. Stop an arm before exceeding its documented experiment ceiling; report the failure instead of risking an OOM.
- **Kill:** no demonstrated bounded representation supports a selected extent. G0 adds focused architecture slices and repeats the failed proof before production. Shrinking the fixed sizes is not a fallback.
- If visual evidence is produced, use compare-screenshots on bounds/residency against the experiment's plan overlay, then an unprimed screenshot-critique last. Use preview-shots for a non-blocking review.

## Delegated to the implementer
Allocation instrumentation, safe experiment limits and current-path probes. Alternative representations need focused followup proofs at G0. Production representation and numeric release budgets are G0 outputs.

## Must stay green
All production behavior; nothing merges. Never modify `../game`.

## Feedback that would change this slice
A changed hardware target or user-approved dimension change requires new measurements.
