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
