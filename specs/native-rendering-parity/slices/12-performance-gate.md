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

## Findings (2026-10-11)

**Pass.** Native is at or below the browser on every measure, on both scenes, in every one of the three runs. It is clearly faster: about half the browser's p95, p99 and GPU time per frame.

Conditions: release export (`release.sh`) against the browser's production build (`graphics-fps`, headless Chromium 148), both scenes, whole reel, against the control `2026-10-11T08-56-34-958Z`. Plugged in (`AC Power` at every run start), M5 Pro, 120 Hz display, 1280×720, 60 warm-up frames. Native uncapped with its window minimised, the browser display-paced. Three rounds interleaved (browser, native market-town, native paris-corner), each run under the GPU lock. Host load average 2.5–3.7. Evidence and scripts are in `throwaway/perf/gate/` (`summary.md`).

**Frame timing** (`web/fps-compare.mjs`, median run; spread over the three runs in brackets):

| | avg FPS | 1% low | p50 ms | p95 ms | p99 ms |
| --- | --- | --- | --- | --- | --- |
| market-town browser | 71.6 (70.5–72.3) | 37.1 | 16.33 | 23.92 (18.59–24.61) | 25.38 (25.29–25.38) |
| market-town native | 115.0 (114.6–116.6) | 67.3 | 8.34 | 13.37 (12.36–13.62) | 14.41 (13.47–15.21) |
| ratio | 1.61 | 1.81 | 0.51 | 0.56 | 0.57 |
| paris-corner browser | 101.5 (94.1–103.8) | 52.6 | 8.34 (8.34–8.42) | 16.83 (16.70–16.94) | 17.78 (17.55–18.06) |
| paris-corner native | 120.0 (120.0–120.0) | 97.2 | 8.25 (8.22–8.25) | 9.18 (9.18–9.20) | 9.57 (9.53–9.57) |
| ratio | 1.18 | 1.85 | 0.99 | 0.55 | 0.54 |

**GPU busy per frame.** Both clients pace at the 8.3 ms display step, so frame time can't show headroom. A 3 s Metal System Trace gives each client's GPU busy share of wall time; times the untraced mean frame interval, that is busy ms per frame. Three traces per side and scene, at about a quarter, half and three quarters of the reel; medians.

| | browser | native | ratio |
| --- | --- | --- | --- |
| market-town | 10.77 ms | 5.93 ms | 0.55 |
| paris-corner | 8.17 ms | 4.47 ms | 0.55 |

- The browser's untraced timestamp queries read 9.47 and 5.87 ms, so the calibration overstates it by 14% and 39% (tracing slows its frames); against those the ratios are 0.63 and 0.76.
- Native's traced frames mostly held the 8.32 ms step, and native has no timestamps to check against.
- Other processes did at most 5.7% of the GPU's work in any trace.

**Plates.** The release export's 24 plates are byte-identical to an editor run's at the same commit, and the editor's to slice 10's.

**One measure failed first, and is fixed.** The first set, and a second with native capped at the display, failed paris-corner p50: native 8.37 ms against the browser's 8.34. Every native run whose window ended on screen read 8.37, and every minimised one 8.24–8.33. Four of six runs in each set were on screen, though the project starts minimised: macOS had brought the window back.
- The fix: the bench minimises its window again just before timing (`parity_run.gd`). Six of six probes and all six gate runs then stayed minimised, and paris p50 fell to 8.22–8.25.
- Native's intervals scatter about ±0.8 ms round the step and average exactly 8.333 ms: it presents every display interval. The browser's rAF intervals are the vsync times themselves. So at the step, presenting's 0.1 ms (slice 10) is enough to move a median.

**Fairness.**
- The browser also runs the live simulation worker and the reel script; native replays recorded publications. The browser draws its own run of the reel, about 8,000–8,300 market-town frames a run; native draws the control's 7,796 records.
- Native drew, per timed frame (mean / max): market-town 920 / 1,696 draws and 8.5 / 14.0 M primitives; paris-corner 638 / 753 draws and 7.05 / 7.51 M primitives.
- Native's content is terrain, grass, sky, haze and post as the browser's, plus the legacy layer:
  - market-town: 600 of 3,050 building shells (those within 1 km of a frame target), with no facade modules; 14,883 catalog props; 21 own-side units, one unposed model each;
  - paris-corner: 103 shells, 101 props and 3 units.
- Native draws no water, scenery (tree lines, forests), structures, effects or scars. The browser draws all of them, plus posed squads, enemies and corpses.
- Not a gate measure: CPU p50 on paris-corner is 1.54 ms native against 0.88 browser (market-town 1.63 against 4.97). The two clocks cover different work. Native's covers its step, frame setup and render submission. The browser's covers its main-thread callback only: its simulation runs in a worker and its GPU submission in Chrome's GPU process.

**Checks.** Native tests green under the lock. Every `graphics-fps` run passed its own checks, including the in-game counter against the recording.
