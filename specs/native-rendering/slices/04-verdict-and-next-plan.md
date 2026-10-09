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

## Checkpoint verdict — 2026-10-09

The evidence supports **keep Godot experimental** and reslice the comparison around a display-backed authored renderer. The browser remains the release baseline.

| Evidence | Browser control | Godot probe | Decision use |
|---|---:|---:|---|
| Workload identity | `menu-reel`, fingerprint `c905ce8c`, two menu scenes | Same scene ids and captures consumed | Passes identity and capture routing |
| Capture stream | 3,541 `market-town` and 1,291 `paris-corner` publication samples with displayed-frame timelines | 1,676,322 raw words validated; semantic unit decoding exercised | Passes authority transfer; no simulation authority moved |
| Displayed frames | Foreground browser run exists in ignored evidence | Current native run is headless/dummy-renderer | Not comparable for performance |
| FPS sample | 13.9 average in the retained browser report | 132.9 process-frame average in the geometry probe | Do not compare; clocks and display paths differ |
| Visual cuts | Browser reference cuts are not frozen as committed named images | Native cuts contain transparent/flat frames and incomplete geometry | Fails visual-equivalence gate |
| Timing envelope | Browser reports CPU/GPU/memory fields | Native reports startup, capture decode and presentation; GPU/transfer/simulation remain null | Reslice required |

The native decoder, map-geometry pass, report envelope and browser evidence export are useful foundations, but the current screenshots cannot answer whether the two renderers show the same battlefield. The missing proof is a foreground Godot run with the authored asset catalog, terrain/props/fog composition, observed units, and matched named cuts.

The focused follow-on is [display-backed authored comparison](../follow-on-display-backed.md). It keeps the GDExtension and capture seams unchanged, adds no gameplay authority, and does not alter `/benchmark?preset=city-contact`.
