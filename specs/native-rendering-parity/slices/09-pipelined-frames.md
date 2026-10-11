# 09 — Pipelined frames

**Contract:** native never makes the CPU wait on the GPU, or the GPU idle between passes, unless a frame genuinely needs it. Uncapped, the GPU is busy for nearly the whole frame, and frame time is close to GPU busy time.

**Seam:** whatever 08 names. Expected owners:
- **The timing loop.** Frames free-run from `_process`, timestamped per frame; no `await frame_post_draw` per frame.
- **The sky** (`environment/`). The radiance cubemap updates only when the light changes. In the reel the sky is fixed and the clouds drift, so bake radiance once (the browser bakes its sky into a lookup texture) and keep only the visible background per frame.
- **Compute passes** (grass build, haze, post). No full device syncs. Barriers only between dependent passes, and buffer updates that don't stall.
- **Godot settings that throttle a frame.** Frame queue size, threaded rendering, and low-processor mode.

**Runnable artifact:** before/after traces and timings in `throwaway/perf/pipelined/`, plus before/after plates.

**Focused gates:**
- From the trace: GPU idle per frame is below 10% of the frame, and no CPU waits on the GPU within a frame.
- Plates are byte-identical, or within the native grass noise floor of 1–2 px of at most 2 levels.
- `native/godot/tests/run.sh` stays green.

**Visual variable:** none. The picture must not change.

**Delegated:** how each wait is removed, provided the picture is unchanged and the trace shows the gap gone.
