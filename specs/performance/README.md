# Performance (placeholder)

**Status:** trigger confirmed; candidate selection is the next user decision.
**Updated:** 2026-10-19.

The current renderer research ledger is [research.md](research.md). It records
the fixed benchmark contract, measured candidates and the next experiment; this
placeholder remains the durable list of deferred work.

## Next Agent Prompt

You are picking up a measured browser frame-time problem. The agreed policy is
reactive and browser-led: choose the smallest candidate that explains the
reported symptom, allow a small secondary regression only when the primary gain
is substantial and recorded, and keep native as a parity check for shared
renderer or shader changes. Update this section before you end each pass.

- [x] A repeatable 1% low regression is measured on the graphics-fps reel.
- [ ] The candidate it points at is chosen with the user, then planned with `write-spec`.

## Current handoff

The canonical production reel was run twice on an uncontended browser at
1280×720. `market-town` averaged 75.9 and 82.2 FPS, but its 1% low was 35.9
and 38.9 FPS; `paris-corner` averaged 107.8 and 110.5 FPS, with 1% lows of
57.2 and 54.9 FPS. The trigger is therefore confirmed for `market-town`.

The frame is not GPU-saturated in these runs: `market-town` GPU means were
11.92 and 10.93 ms, while CPU p99 was 21.16 and 18.47 ms. The next pass must
identify the CPU/frame-pacing owner before changing the renderer. Candidate
choices are static-model packing, another main-thread span found by a profile,
or a measurement fix if the low is caused by pacing rather than work.

## Where the frame stands

The battle draws one pixel per CSS pixel on any display, and the system draws the
cursor, so neither a high-density screen nor a slow frame shows up as input lag.
Measured on 2026-10-10 (M-series Mac, headless Chromium, medium generated battle,
1600×1000): about 60 fps at 1× and 2×, GPU 7.8 ms a frame on average. In a
production build the main thread is idle about three quarters of the time;
React costs about 0.3 ms a frame and the renderer's JavaScript about 2 ms.

## Candidates

### GPU memory: the whole roster is resident

A small map held 2.3 GB of live GPU memory: 1,076 MB of unit and prop meshes at
every detail tier, 594 MB of wreck meshes, 198 MB of frame targets, 140 MB of model
textures and about 170 MB of unlabelled textures. A battle installs art for every
unit its session's catalog names, all three factions (311 kinds of model), though
only two sides can take the field: in a US battle, the eastern and European meshes
were 526 MB and 464 MB. The JavaScript heap is small (64 MB after collection).

Options, cheapest first, each with its cost:

- **Install the two fighting factions only.** About 0.5 GB less. The session catalog
  must be scoped to the battle's sides, and anything that can appear without being
  either side's (a neutral unit, a lab) must still be covered.
- **Upload a vehicle's wreck when it first dies.** Up to 0.6 GB less. A wreck could
  appear a moment late, or the live mesh must stand in until it does.
- **Store vertices more compactly** (quantized positions, packed normals). Could
  roughly halve all mesh memory; a renderer and bake change.

No problem has been seen on the development machine. The risk is a smaller
machine's GPU memory.

### Static models repack whenever a unit moves

The model layer packs units and statics (buildings' props, scenery) in one counting
sort that makes one instanced draw per mesh. During a battle units move every
frame, so every frame repacks every static in view too, though statics change only
with the camera: about 4 ms of main-thread time a frame. Packing statics only on a
view change needs two record regions, two runs per mesh, separate card ranges and a
staleness rule covering the view, the static list, hidden props, newly installed art
and per-frame extra statics: an estimated 150–250 lines in a hot path, where a stale
record draws a prop in the wrong place. The main thread is not the limit today.

### Asset loading holds the main thread

During a battle's loading, bundle verification re-encodes each decoded bundle,
texture pixels included, in JavaScript to recheck its hash, and hashes each file
twice: 2.3 s of main-thread work, including one six-second frame. The total
startup time is within its budget; only the frozen frame would justify work, by
verifying from the parts without re-encoding or moving loading into a worker.

## Dismissed

- **React re-rendering the battle view on each publication:** about 0.3 ms a frame
  in production. The 4 ms seen first was the development build.
- **Decompression, audio decoding, the simulation:** already native or off the main thread.
- **Per-frame `elementFromPoint` lookups:** about 0.1 ms a frame.

## How these were measured

Each number comes from a throwaway browser probe, not a kept tool:

- **Frame cost:** frame intervals from `requestAnimationFrame`, GPU time from the
  lab's frame statistics, and a sampled CPU profile (the DevTools protocol) on a
  running battle, read for idle share and time by function or by bundle file.
  Compare development and production builds separately; React and dev tooling
  inflate the first.
- **GPU memory:** wrapping the device's buffer and texture creation before the page
  loads, keeping each live allocation's label, size and creating stack, then
  grouping live bytes by label once a battle has loaded.
- **Heap:** after a forced collection. `performance.memory` counts uncollected
  garbage and misreads a 64 MB heap as 1.5 GB.

The player's own copied diagnostics report (settings, Copy diagnostics) carries
frame intervals, long frames with their scripts, input delay, the GPU adapter,
canvas size, GPU frame time and allocations, and recent errors.
