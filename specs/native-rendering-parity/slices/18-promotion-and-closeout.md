# 18 — Whole-frame verdict, promotion and closeout

**Contract:** a whole-frame comparison across every plate of both scenes, with every earlier variable already green. Slice 12's performance gate is re-run with all content drawn. Native becomes eligible for promotion (`comparison_ready` true) only when both pass.

**Runnable artifact:** full contact sheets of every plate pair from one fresh control run, plus the final performance table.

**Gates:**
- **Whole frame.** `compare-screenshots` over every pair, then an unprimed `screenshot-critique` of the whole set, then `preview-shots`.
  - Every remaining difference is named. A named difference is explained by an inherent platform behaviour, such as compiler float rounding, or it fails the gate.
- **Performance.** Slice 12's measurement on the release export, three runs, plugged in: native no slower than the browser on any measure, on both scenes.
- **Completeness.** The native report has no unexplained fallback or omission.
- **Determinism.** Rust digest and replay are unchanged.
- **Promotion.** `comparison_ready` is true only if all of the above pass. Otherwise the report carries a blocker naming the first failed gate, and native stays experimental.

**Closeout:**
1. Run the whole repository check and verification suite once (`bun run check`, `bun run verify`). Name any failure unrelated to this spec rather than repinning it.
2. Run the full `review` over the spec's whole diff.
3. Consolidate `choices.md`.
4. Archive with `close-spec`, keeping the final control, cuts, performance tables and critiques.

The browser stays shipped either way. Promotion only makes native eligible; it ships nothing.
