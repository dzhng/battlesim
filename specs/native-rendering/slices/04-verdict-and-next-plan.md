# Slice 04 — Verdict and next plan

## Contract unlocked

Produce an evidence-backed choice about whether Godot advances, stays experimental or is rejected. Compare only equivalent browser and Godot outputs and retain the browser release baseline.

## Evidence table

Repeat both clients sequentially on the pinned Mac with the same build mode, resolution, quality, scene identities, warm-up, duration and GPU serialization. Report distributions, not a peak run: average FPS, 1% low, absolute minimum, maximum, frame-time chart, CPU/GPU/presentation, transfer, startup, shader compile, memory, visual gaps, code reuse, maintenance ownership and platform reach. Keep offscreen, submission-only and displayed-frame numbers distinct.

## Runnable artifact

Archive the accepted manifest, captures, reports, comparison charts, candidate failures and choices in the spec evidence area or ignored scratch area according to LFS policy. Write a short verdict and a focused follow-on spec for the chosen direction. If evidence is insufficient, reslice around the missing measurement instead of widening implementation.

## Verification

Run narrow candidate reports first, then full repository `check` and `verify` once at closeout. Re-run the existing synthetic benchmark to prove its workload remains separate and healthy. Use compare-screenshots and an unprimed screenshot-critique for the final visual matrix, and preview-shots for human inspection. Review is non-blocking; record rationale if no response arrives.

State whether Godot has broad gameplay/visual-target parity, whether the selected binding is maintainable, whether a native WebGPU or Rust `wgpu` probe is justified, which platforms remain open, and who owns UI/audio/asset integration. Do not turn a measured advantage into a production commitment without a new implementation spec.

## Must stay green

All prior slices, browser release behavior, replay/digest parity and the existing synthetic benchmark.
