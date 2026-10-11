# 12 — Performance gate

**Contract:** on both reel scenes, plugged in, at 1280×720, in release builds (11), native's frame time is no worse than the browser's.

**Seam:** slice 07's harness, unchanged.

**Measures:**
- average FPS, 1% low, p95 and p99 frame time, uncapped if both clients can be; otherwise both capped at the display rate, with GPU busy ms as the headroom measure;
- the ratio of native to browser on each.

**The bar:**
- **Hard:** native is not slower than the browser on any measure, on either scene.
- **Expected:** native is clearly faster. If it isn't, 08's anatomy must explain why the remaining cost is inherent.
- **Fairness, stated in the report:**
  - the browser also runs the live sim worker;
  - native replays recorded publications;
  - native still lacks effects, most infantry and detailed models.

  Phase 3's content will add native cost, so record the content each side drew.

**Runnable artifact:** `throwaway/perf/gate/summary.md` and the traces behind it.

**Focused gates:**
- Three runs per side, with the spread reported.
- Plates within the noise floor.
- Native tests green.

**After the gate:** phase 3 (models, effects, promotion) resumes. Its slices inherit the budgets: a new layer reports its own pass cost, and the gate is re-run when phase 3 closes.
