# Slice 04 — Auto-detect, first launch, and fatal startup

## Contract unlocked

Auto-detect runs a bounded canonical graphics workload on first launch and on explicit rerun, records valid evidence, selects the highest tier meeting 45 average FPS and 30 FPS 1% low, and fails loudly when the required measurement cannot complete.

## API seam

Reuse the existing graphics-test reel and recording summaries. Add a bounded probe adapter that can run each candidate with warmup excluded, finite frame/sample minimums, timeout/cancellation, adapter identity, viewport, build, profile, and benchmark fingerprint. Timestamp-query GPU data is preferred; RAF frame samples remain required for average/1% low and must be marked display-paced. Never treat missing, failed, cancelled, NaN, or insufficient data as zero.

On first launch, block play behind a progress/error surface until Auto-detect completes. Persist the concrete effective tier and evidence. Rerun is available in Settings. Missing WebGPU, adapter/device failure, device loss, or unusable benchmark is a fatal startup screen with reason and Reload. Fix the current menu-boundary behavior that can otherwise leave a blank backdrop alive after a required resource failure.

## Verification

Write deterministic resolver/benchmark-adapter tests for threshold edges, display refresh-rate caps, missing timestamp query, timeout, failed scenes, incomplete samples, and no passing tier. Add browser scenes for first launch, persisted launch, rerun, progress, and fatal failure. Run a real production benchmark on the Windows AMD adapter and retain the report artifact.

No screenshot is accepted without `screenshot-critique`; use `compare-screenshots` for startup/menu before-after and `preview-shots` for human review.

## Delegated freedom

The probe may run hidden in the menu backdrop or in a dedicated loading route, but it must reuse the canonical workload, remain bounded, and block play until valid completion.
