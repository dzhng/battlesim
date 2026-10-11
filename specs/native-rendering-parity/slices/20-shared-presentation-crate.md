# 20 — Shared presentation crate

**Contract:** the per-frame presentation derivations live once, in a Rust crate, `crates/presentation`. The browser runs them through WASM and native links them, and both clients use the same code.
- **Interpolation:** per-soldier blending.
- **The pose feed and pose driver**, including articulation, gait and the corpse limit.
- **Launches and the effect feed.**
- **The effect frame:** seeded bursts, dust, smoke and cast lights.
- **Cook-offs.**

The TypeScript originals and native's ports are deleted.

**Seam:**
- `crates/presentation` depends on `contract` and on slice 19's reader.
- It produces per-frame plain data: posed model instances with joint or part transforms, effect instances by shape, and cast lights.
- Each client draws that data. The browser keeps its TypeGPU passes and native its Godot passes.
- **Tuning data** (`presentation.effects`, pose tuning, unit facts) is read from the same fixture and catalog data both clients already load.
- **Seeded randomness:** Mulberry32 and `hashString` live in the crate. JavaScript's number-to-text behaviour, where the seeds depend on it, becomes a defined function that both sides use.

**Research first:**
- Per-frame call overhead and data marshalling from WASM on the main thread, for a battle's worth of units and particles.
- Whether outputs can be written straight into the browser's instance buffers (shared memory views).
- How the browser's renderer packages (`packages/battle-renderer`) consume them without a second copy.

Write the findings into this file before building.

**Posing (18b question 1, decided 2026-10-11): Rust builds the joint palettes.** The baseline below found the browser GPU-bound, with posing at about 0.06 ms per frame on the GPU against an estimated 0.05 ms in native Rust, so neither choice moves frame time by even 1%. Rust palettes delete both GPU pose kernels, which is less code. Superseded procedure, kept for the record: before building poses, measure whether the browser is CPU- or GPU-bound today. Prototype Rust-built joint palettes against the current GPU kernel fed by Rust, and keep the faster. Record the numbers here.

**Order:** interpolation and poses first (gate: slice 14's pose vectors, then the model counts), then effects (gate: slice 17's effect vectors and counts).

**Order: web first.** Land the change in the browser first. Measure before and after with slice 07's harness on the graphics test and a medium battle: average FPS, 1% low, p95 and p99, main-thread CPU and GPU timestamps, three runs each. Only then switch native to the same code.

**Focused gates:**
- **Parity with today.** Before any TS module is deleted, the Rust output equals today's TS output on the existing vector tests and on every plate's counts.
- **The shipped browser is unchanged in what it draws.** Its plates are byte-identical to before, and digest and replay unchanged. Graphics-test frame time and main-thread CPU, across three runs, show no meaningful regression; a small measured cost is acceptable for less code.
- **Less code.** The slice reduces net lines across Rust, TypeScript, GDScript and tests (excluding comments and fixtures), reported as added, deleted and net per language. A slice that grows the total is reworked before it lands. This is the phase's point: one owner instead of an original, a port and a compare test.

- **Native.** Plates and counts unchanged, and CPU per frame reported.

**Delegated:** the crate's internal structure.

## Baseline: CPU or GPU bound (2026-10-11)

The first half of 18b question 1. Measurement only; nothing was changed.

**Verdict:** relative to the headless 120 Hz rAF steps, every workload is **GPU-bound**, except market-town, which is mixed. The pose kernel is not what bounds any of them: on the GPU it costs about 0.06 ms a frame, and done in Rust on the CPU it would cost about 0.05 ms (native) for the battle's 1,909 joints. Moving posing either way changes frame time by under 1%. Decide question 1 on code and parity, not speed, as long as posed bodies stay in the low thousands.

**Conditions.** Plugged in. Production build. Headless Chromium 148 on `apple metal-3`. Everything ran under the GPU lock, three runs per workload. Pacing is headless rAF, which snaps to 8.33 ms steps, so a frame that misses 8.33 ms lands at 16.67 ms.
- **Reel:** 1280×720, the `graphics-fps` scene with `fps-compare.mjs`.
- **Medium battle:** 1600×1000 at DPR 1. `/battle?type=mixed&size=medium&seed=1&faction=us` opens on an empty preparation, so a scratch probe (`throwaway/cpugpu/probe.mjs`) plays it unattended:
  - it buys with `confirm_purchase` (7 units placed, 39 soldiers; one placement was refused) and starts with `ready`;
  - it advances 1,800 ticks, orders `attack_move` to the farthest objective, then advances 2,400 more;
  - it aims the camera at 90 m on the densest own cluster (36 posed bodies) and measures live.
  - A medium battle today is this small, a 1,000-credit wallet a side. The performance README's medium battle predates the preparation phase.
- **WASM** was rebuilt for this tree. wasm-pack's `wasm-opt` step failed, so the bindgen output ran unoptimised by wasm-opt. The simulation runs in its worker, off the measured thread.

**Frame intervals** (ms; per-run values, or the median and range across runs):

| workload | avg FPS | p50 | p95 | p99 |
|---|---|---|---|---|
| market-town (whole scene, 3 runs) | 63.8 (63.5–67.3) | 16.65 | 25.08 | 26.25 |
| paris-corner (whole scene, 3 runs) | 95.9 (95.5–103.5) | 8.38 | 16.89 | 17.95 |
| medium battle (10 s window, 3 runs) | 59.7 / 59.6 / 58.0 | 16.67 ×3 | 24.6 / 25.0 / 25.0 | 25.1 / 25.2 / 25.3 |

Run 1 of the reel was 6–8% faster in both scenes; the cause wasn't found.

**Main-thread CPU per frame** (ms). The first column is the graphics test's own rAF-callback time, unprofiled, over the whole scene. The rest come from a CDP sampled profile (200 µs) over 6 s, attributed through source maps. Pose, effect and decode are inclusive; the other columns are by innermost owner.

| workload | rAF callback p50 / p95 | busy (profiled) | renderer JS | React | `poseDriver` + poseFeed | `effectFrame` | decode | app / native / GC |
|---|---|---|---|---|---|---|---|---|
| market-town | 5.75 / 8.7–16.1 | 8.0–8.9 | 7.0–7.7 | 0.06 | 0.06–0.08 | 0.12–0.15 | 0.16–0.22 | 0.6 |
| paris-corner | 1.12 / 1.2–1.4 | 1.2–2.9 | 0.7–1.7 | 0.04–0.08 | 0.03–0.07 | 0.09–0.17 | 0.07–0.17 | 0.3–0.7 |
| medium battle | (not recorded) | 3.9–4.2 | 1.46–1.55 | 0.47–0.55 | 0.06–0.08 | 0.05–0.06 | 0.12–0.14 | 1.7–1.8 |

- In market-town the renderer's JS is the model layer repacking statics: `modelLayer.ts` takes 3.9 ms and `modelDetail.ts` 1.4 ms, over 11,716 instances. This is the "static models repack" candidate in `specs/performance/README.md`.
- In the battle the largest renderer item is `fogVisibility.ts` at 0.5 ms. The battle's main thread is about 75% idle.

**GPU per frame.**
- `stats().gpu` (timestamp brackets), mean / p95 ms: battle 4.5–4.8 / 13–15; market-town 6.0–6.1 / 8.4; paris-corner 5.4–7.2 / 11–14. The graphics test's whole-scene means were 9.8 for market-town and 6.0 for paris-corner.
- **These under-report on this GPU.** A Metal System Trace of Chrome's GPU process, with Dawn's `use_user_defined_labels_in_backend` so passes carry their WebGPU labels, gives:

| workload (traced, one run) | GPU busy share | busy ms per frame | `world` fragment | `world` vertex | shadows (4 cascades) | pose kernel |
|---|---|---|---|---|---|---|
| medium battle | 0.70 | 13.2 | 10.4 | 0.57 | ~0.1 | 0.064 |
| market-town | 0.79 | 7.9 | 4.0 | 1.5 | ~1.6 (vertex) | merged into a compute encoder |
| paris-corner | 0.83 | 9.9 | 5.0 | 2.7 | ~2.1 | 0.061 |

- Frames are counted by `compose` or `pose-kernel` encoders. Tracing slowed the frames by about 10%, so read the shares, not the absolute ms.
- `stats()` has no per-pass split.
- `timePoseKernel` (the kernel alone, 50 reps):
  - 0.03–0.05 ms at the frame's 8–36 bodies;
  - 0.04–0.05 ms at 1,000 bodies (53k joints);
  - 0.05–0.08 ms at 4,000 bodies (212k joints).
  - The kernel is dispatch-bound: about 8 ns per extra body.

**Why each verdict.**
- **Battle: GPU.**
  - The main thread is busy 3.9–4.2 ms of a 16.7 ms frame.
  - The GPU is busy about 13 ms, above the 8.33 ms step, and the `world` pass takes most of that.
  - Hiding the canvas left the main thread's work unchanged, dropped the GPU busy share from 0.72 to 0.48, and raised the frame rate from 60 to 75 FPS.
- **Paris-corner: GPU.** Every long frame (670–989 per run) had under 6 ms of rAF-callback CPU, and the GPU is busy about 10 ms a frame.
- **Market-town: mixed, mostly GPU.**
  - Of about 5,200 long frames per run, 11–42% had more than 8.33 ms of rAF-callback CPU; the rest had less.
  - The CPU sits at the edge: the profiler's own overhead dropped the window from 94 to 63 FPS.

**Posing on the CPU instead.**
- Measured per joint (`throwaway/cpugpu/posebench.mjs` and `posebench-rs`), in instructions retired net of a zero-iteration baseline, with wall time:
  - the shipped TS path (`sampleClip` → `worldTransforms` → `· inverse_bind`): 6,165 instructions, 238 ns;
  - a tight typed-array JS loop of the kernel's math: about 3,100 instructions, 97 ns;
  - the same loop in Rust, native release: 330 instructions, 27 ns.
- WASM wasn't measured; expect 1.5–2× native.
- Joints per frame: battle 1,909 (36 soldiers × 53 joints); market-town 652; paris-corner 584.
- Cost at the battle's 1,909 joints:
  - TS: 0.45 ms;
  - Rust: 0.05 ms native, about 0.08–0.10 ms in WASM;
  - 18b's 3,600 joints: about 0.1–0.2 ms in WASM.
- The palette upload is unchanged: the frame already writes `paletteMatrices × 64 B` (122 KB in the battle) every frame.
- The GPU kernel today: about 0.06 ms a frame. That is under 0.5% of the bounding GPU time, and it would be freed by the move.
- The GPU kernel scales far better: at 4,000 bodies it takes 0.08 ms, where CPU Rust would take about 5.7 ms. This matters only if posed meshes reach the thousands.

**For slice 07 and this slice's gates.** On this GPU, `stats().gpu` reads about a third of the GPU's real busy time in the battle. Compare GPU cost with a labelled Metal trace's busy share, not the timestamp mean.
