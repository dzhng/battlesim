# 08 — Native frame anatomy

**Contract:** a measured account of where a native frame's time goes, pass by pass, set beside the browser's. It names the cause of the 77 ms frames before anything is changed.

**Evidence that opened it** (2026-10-10, market-town, 1280×720, M5 Pro, on battery, Godot editor DEBUG build):
- the Metal HUD showed 14 FPS and 76.7 ms of GPU time per frame;
- btop showed the GPU at about 62% and the CPU nearly idle.

Long frames on an idle machine mean the frame is **waiting**: the CPU and GPU are serialised, or the GPU idles between dependent passes. They don't mean the frame does too much work.

**Seam:** slice 07's harness. Two complementary methods on the same frames:
- **Timeline.** A Metal System Trace of each client: the passes per frame in order, their durations, the gaps between them, and every CPU wait on the GPU.
- **Ablation.** Turn one native feature off at a time with a scratch switch (nothing committed) and record frame ms and GPU busy ms. In order of suspicion:
  1. the timing loop's per-frame `await RenderingServer.frame_post_draw`;
  2. the sky radiance cubemap re-rendered each frame (`Sky.process_mode` is unset, so it updates whenever `sim_time` changes);
  3. the grass compute build dispatched through `call_on_render_thread` every frame;
  4. the haze and post `CompositorEffect` compute passes;
  5. the terrain fragment shader, which reassembles each table word from four RGBA8 texels while walking the plot tree;
  6. shadow cascades with grass casters;
  7. 4× MSAA.

**Runnable artifact:** `throwaway/perf/anatomy/` holds both clients' traces and a table: pass or feature → GPU ms, gap ms, CPU wait ms, browser against native. The findings are written into this slice file.

**Focused gates:**
- Every ablation row is the median of three runs, with the spread shown.
- The sum of native passes plus gaps agrees with the measured frame time within 10%; otherwise the account is incomplete.

**Out of scope:** fixes. A fix found along the way is noted with its measured effect and lands in 09 or 10.

**Delegated:** how ablation switches are wired, provided none is committed.

## Findings (2026-10-11)

Measured with slice 07's harness, editor (debug) build, plugged in, M5 Pro, 1280×720. Evidence is in `throwaway/perf/` (`compare2/`, `anatomy/`, `window/`).

**The 77 ms frames were two things.**

1. **Waiting, from the run loop and the window.** The old loop awaited the main loop's draw. Godot's main loop skips drawing, and sleeps, while its window is minimised or covered (`can_any_window_draw`). So a hidden window stalled a run outright: 18 minutes reproduced, the main thread in `nanosleep` in 98% of samples. A shown window paid presentation, and the GPU idled about half of each frame in the first trace. Slice 07 fixed the harness (`force_draw`), so this is no longer in the numbers below. Presenting to a shown window still costs 25–26 → 37–48 ms a frame on the same 300 records: the shipped client's cost, reported apart.
2. **Work, from the legacy map layer, multiplied by the shadow cascades.** With the harness fixed, the GPU is saturated. In a traced window, frames overlap on the GPU and its idle time is about 2% of the frame. About 81% of GPU time is vertex work in six render encoders: by their shape, the depth prepass, four shadow cascades and the opaque pass.

**Where the draws and primitives come from** (Godot's own counts per frame, market-town records 1000–1299; counts don't move with machine load):

| Switched off | Draw calls | Objects | Primitives | Frame ms (median; spread) |
| --- | --- | --- | --- | --- |
| nothing | 19,922 | 30,066 | 18.0 M | 39.8 (25.1–41.9) |
| shadows | 3,734 | 9,151 | 7.6 M | 14.7 (13.3–18.1) |
| props (legacy, catalog models in MultiMeshes) | 17,169 | 27,313 | 5.3 M | 18.3 (11.5–25.8) |
| units (legacy, unposed catalog models) | 2,911 | 2,911 | 16.7 M | 31.4 (25.2–32.7); CPU 7.0 → 2.8 ms |
| buildings (legacy shells) | 19,772 | 29,916 | 18.0 M | 30.1 (26.6–34.3) |
| the whole legacy layer | 8 | 8 | 4.0 M | 9.1 (8.3–12.8) |

- **Units carry 85% of draw calls and 90% of objects.** Each unit is an unbatched catalog model of many parts, drawn in every pass.
- **Props carry 71% of the primitives.** The catalog models are drawn at full detail with no LOD.
- **The shadow cascades multiply both:** turning them off removes 81% of draws and 58% of primitives.
- **Terrain, grass, sky and post together** are 8 draw calls and 4 M primitives. With the legacy layer off the frame takes 8.3 ms.
- **The whole reel is heavier than this window:** market-town averages 20,023 draws and 34.2 M primitives a frame, at a 75 ms median frame.

**The suspects the slice listed, measured:**
- the per-frame await: replaced (above);
- the sky radiance cubemap: not re-rendered per frame (the sky shader reads no time and no per-frame parameter);
- grass build, haze, post, terrain, MSAA: each moved frame time by less than the run-to-run noise in this window (about ±7 ms under the host load), so each is at most a few ms and is not the cause.

**Gate.**
- Passes plus gaps against the measured frame: a whole traced bench of 100 records gave a median trace span of 39.5 ms, against the same run's measured median interval of 40.3 ms (2% apart).
- Tracing itself slows native frames: 40 ms traced against 25–26 ms untraced on nearby records. Traces give proportions; untraced runs give the ms.
- Every ablation row is the median of the runs listed, with the spread shown. Host load swung from a load average of about 184 to 9 across the three passes, so the baseline itself spreads 25–42 ms. Only shadows, props and the whole legacy layer separate from it beyond the spread; every other row (sky, sky light, haze, post, grass build, grass, terrain, MSAA, buildings, units' GPU share) overlaps it. The counts above are exact and load-free.

**Browser beside it.** The browser's GPU, traced on the same scene, was busy 72% of the wall time with 28% idle, and timestamp queries measure 6–10 ms of GPU per frame. Its frames snap to the 120 Hz display steps (8.3, 16.7 and 25 ms), so its average FPS swings with how many frames make 8.3 ms. Browser draw counts are not exported (`FrameStats` has partial counts only).

**For 09 and 10, not fixed here:**
- the legacy layer's units should batch (instancing per model) and props need LOD or impostors; with the whole layer off the window's frame drops from about 40 ms to 9;
- shadow casters need culling to what each cascade sees;
- all of this is phase 3's ground (slice 13 replaces `legacy_map_layer.gd`), so 09 should first decide whether to tune the scaffolding or measure without it.

A shipped client pausing while its window is hidden is normal game behaviour: a phase 3 question, not a 09 fix.
