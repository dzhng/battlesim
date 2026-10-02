# Performance

The one gate is the user's floor: **30 FPS average at the default camera, 1920×1080**, measured by the benchmark scene. Everything else is recorded, not gated. Keep the headroom by measuring every change that can cost a frame, not by guessing.

## Measuring

- **Measure a feature paired, never as two absolute runs.** Interleave it on and off on one machine, in batches longer than the timer's window, and take the median of the differences. Other load on the machine swamps small differences, and the tails (p95, p99) are contention, not your change.
- **A change with no switch is paired as two source trees,** each served to its own browser, with batches interleaved between them.
- **One render at a time.** Renders share the machine's one GPU; take the shared lock per render so jobs take turns.
- **GPU time is the whole-frame total.** There is no trustworthy per-pass split on Apple GPUs; isolate a pass with the paired switch.
- **Force a redraw every frame while probing.** The lab draws on demand, so an idle frame reports only vsync.
- **Prove the GPU.** A headless shell can fall back to software rendering, whose timings are void. Record the adapter with the numbers.
- **Report memory with every time.** Memory grows faster than time, and each new full-screen multisampled target is tens of megabytes.
- **Compare benchmark rows only within one scenario and camera tour.** The widest camera is always the most expensive phase.

## What keeps a frame cheap

- **Recompute only on change.** Rebuild for the eyes that moved, the tiles that changed, the view that crossed a step. Trace identity through adapters first: an equal list rebuilt upstream reopens every cache downstream.
- **Nothing allocates per frame on hot paths.**
- **Cost follows what is in view, not what is on the map.** Cull the view by the frustum and the casters by their shadows; draw far things as merged static ranges; install only what the battle draws.
- **Level of detail is chosen by projected size,** and casters can be a tier coarser than what they cast for.
- **Never loop over authored shapes per fragment or per instance.** Index them once, by where they are, and read the index.
- **Overlays that rebuild are coarse and few-vertexed.**
- **Large buffers reach the page through a stable feed,** never through props a UI framework copies or diffs.

## Traps

- **Alpha-tested cards for foliage:** they need a discarding prepass and caster, and they shimmer under multisampling.
- **Saving a little by thinning what frames the scene** (the horizon, the backdrop). It costs more in the picture than it saves.
- **Copying another project's renderer wholesale.** Borrow ideas; measure them here.
