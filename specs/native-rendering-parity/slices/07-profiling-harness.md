# 07 — Profiling harness

**Contract:** one command per client measures the same scene, frames, window and power source, and reports the same numbers from the same code. A GPU trace and a CPU profile come from the same tools on both sides.

**Seam:**
- **Frame timing.**
  - Browser: the graphics test's own recording. These are `requestAnimationFrame` intervals of drawn frames (`apps/battle-lab/src/benchmark/reelRun.ts` → `BenchmarkRecording`), the same render-loop timestamps the in-game FPS counter and the diagnostics report read, plus `stats().gpu` from timestamp queries.
  - Native: a timing mode in `parity_run.gd` that steps every frame record with no readback, vsync off and `Engine.max_fps = 0`. It writes **raw per-frame intervals** into the report.
  - **One summariser**, in TS, computes every summary number for both sides: average FPS, 1% low, min, max, p50/p95/p99 frame time. Only the interval clock differs between the sides.
- **GPU trace.** `xctrace record --template 'Metal System Trace'` attached to Godot and to Chrome's GPU process. Find xctrace via `DEVELOPER_DIR=/Applications/Xcode.app/Contents/Developer`, because `xcode-select` points at the command-line tools. From the trace, a small parser reports per frame:
  - GPU busy ms;
  - GPU idle gaps;
  - each encoder's duration, labelled by pass. Native labels its passes with `RenderingDevice` debug labels, the browser with WebGPU `label`s.
- **CPU profile.** `xctrace record --template 'Time Profiler'` on the same runs: the main and render threads' heaviest frames.
- **Pacing.** Both clients are compared under the same pacing: uncapped, or both capped at the display's rate. The Metal HUD log (`MTL_HUD_ENABLED=1 MTL_HUD_LOG_ENABLED=1`) is a cheap cross-check of GPU ms, not the source of truth.
- **Conditions in every report:** power source (plugged in for comparisons), the build (Godot editor DEBUG or release export template; browser dev or production), display refresh rate, and adapter.

**Runnable artifact:**
- `throwaway/perf/<run>/` holds both clients' summaries, traces and a `summary.md` table.
- Commands are documented in the native README and the benchmark README.
- Nothing appears on the user's screen. Godot renders off-screen and Chromium headless; if macOS clamps a window on screen, say so.

**Focused gates:**
- The summariser's unit tests.
- On one browser run, the in-game FPS counter's 1 s readings agree with the recording's average over the same stretch.
- Three runs per side, with the spread reported. A large spread is explained before any comparison.

**Out of scope:** making anything faster.

**Delegated:** the trace parser's internals and the run folder's layout.

**Known:** Chromium's `--disable-frame-rate-limit` hangs the graphics test in battle preparation, inside the model layer. `--disable-gpu-vsync` alone doesn't uncap rAF. If the browser can't be uncapped within a time box, compare both capped and lean on GPU busy ms for headroom.
