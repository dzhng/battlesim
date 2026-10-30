# 12 · GPU frame timing: one owner

**Contract.** `renderer_core::timing::FrameTimer` serves both the browser's
rolling statistics and native's per-frame bench, and the bench's copy goes.

## Today

- Core (`crates/renderer-core/src/timing.rs`, used only by the browser) resolves
  inside the frame's encoder, never blocks, drops frames when its readbacks are
  busy, and keeps a 240-sample window.
- Bench (`native/client/src/bench.rs:44-179`) resolves after the frame completes
  because Metal returns stale end timestamps otherwise
  (`native/client/README.md:86-92`). So it needs a query set per slot, blocks,
  measures every frame and files each against the caller's frame index.

## Seam

- `FrameTimer::new(device, queue, Resolve::InFrame | Resolve::AfterCompletion)`,
  with per-slot query sets in both modes. The bench keeps its pacing, waits,
  coverage rule and report; it calls the core timer in `AfterCompletion` mode.
- Delete `bench.rs:44-179`.
- **Check the browser** (map unknown 6): run `graphics-fps` against a known
  workload in both modes. If the in-frame numbers are stale on Metal, the browser
  switches to `AfterCompletion` and that is a named decision in `choices.md`.

## Verify

- Failing first: a test that each sample is attributed to its own frame across a
  delayed completion and across `reset()`.
- `native/client/tests/bench_cli.rs`; one `--bench` run whose numbers match base
  within noise; the browser check above. No plates: timing draws nothing.

**Delegated:** the result API and completion plumbing; a non-blocking
`AfterCompletion` if it doesn't deadlock.
**Stays green:** browser responsiveness, bench warm-up and coverage rules.
