# Rust presentation frame generation

## Contract

`Presentation::frame` must emit the same `Frame` records for the same
publication, camera, UI and generation. Browser and native replay digests remain
unchanged; this slice changes only temporary storage ownership.

## Pass

Reuse `Loaded`'s per-frame scratch vectors for high-air shadows and x-ray unit
membership. The vectors are cleared and repopulated in the same iteration order
as the previous temporary collections, so filtering, colours, batches and picks
retain their existing semantics.

## Proof

- focused: `cargo test -p presentation`
- parity: native captured replay/count checks when available
- browser: rerun `graphics-fps` only after the Rust contract is green

## Follow-up pass

`BuildingWalk::select` now double-buffers its module and tier lists and reuses
the count vector. The active and scratch lists swap after comparison, so the
previous output remains available for exact equality while the next walk keeps
its allocations. The browser and native `Frame` records are unchanged.

Proof for this pass:

- focused: `cargo test -p presentation`
- native control: 1.14 ms CPU p50 / 1.45 ms p95
- native reuse: 1.05–1.06 ms CPU p50 / 1.30–1.52 ms p95, identical drawn counts
- browser `paris-corner`: 111.4 FPS average, 57.6 FPS 1% low
- browser `market-town`: 83.2 FPS average, 39.1 FPS 1% low

The native CPU reduction is sufficient to retain this output-equivalent pass.
The `market-town` 1% low remains below the research floor, but the user accepts
that result for the current player-facing target. Further work needs a new
measured symptom before adding complexity.
