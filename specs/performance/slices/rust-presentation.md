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

## Next

Measure whether the reduced allocation churn changes the accepted browser reel;
if it does not, keep the output-equivalent change only if native profiling shows
an allocation benefit, otherwise revert and profile the next loaded-frame owner.
