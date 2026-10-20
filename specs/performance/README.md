# Performance

**Status:** XL 5v5 stress rendering is now the active performance limit. The Rust presentation allocation-reuse passes and browser reel remain green; the culling audit confirms dense in-view props; the distant-car mesh has a measured GPU gain and passes production endurance.
**Updated:** 2026-10-20.

The current renderer research ledger is [research.md](research.md). It records
the fixed benchmark contract, measured candidates and the next experiment; this
README owns the active handoff and acceptance state.

## Next Agent Prompt

Production Metro XL at 100 units per side is the active target. Keep the
simulation unchanged and accept renderer complexity only for a measured gain
with preserved images. The earlier market-town CPU passes are accepted; their
evidence lives in [research.md](research.md).

- [x] Establish the XL stress workload and reproduce expensive GPU frames.
- [x] Separate terrain-caster, model-caster, and other shadow costs with settled,
  interleaved samples at the actual production Low resolution.
- [x] Test and reject union-of-cascades prop filtering: no repeatable GPU gain.
- [x] Separate prop and unit/remains shadow cost: props dominate this opening.
- [x] Measure and reject the single-joint shader branch: no consistent gain.
- [x] Reject per-cascade prop lists: no consistent GPU gain.
- [x] Isolate solid/cutout cost: solid geometry dominates. Reject discard and
  position-only depth shader variants; neither yields a consistent gain.
- [x] Rank prop appearances: parked cars dominate (7.2 million triangles).
- [x] Audit actual selected bounds against the GPU camera: 99.6% intersect
  the view; parked cars cover only 0.8–2.2 rendered pixels each.
- [x] Verify near/far car appearance and measure a controlled 12% GPU saving.
- [x] Confirm the production live/late-state run and review the car pass.
- [x] Finish asset validation (empty stale bundle directory removed).
- [x] Commit the car pass with formatting, lint and file-size hook checks.
- [ ] Verify images, batch updates, resource reset, browser/native consumers,
  and steady-state performance before accepting the candidate.
- [ ] Reassess the remaining GPU limit, review, and complete closeout checks.

## Current handoff

The corrected paused-opening probe uses a 1280×800 CSS viewport, persisted
production Low, and a confirmed 960×600 target. Baseline GPU cost is about
32 ms. Omitting terrain shadows saves only about 0.4 ms; omitting model-layer
shadows saves about 6–7 ms; omitting all shadow encoding saves about 9 ms.
These are diagnostic omissions, not acceptable optimizations, and their deltas
are not additive. Building/scenery shadow deltas were near noise in this view.

All renderer candidates tested so far were rejected for inconsistent gains:
union/per-cascade caster lists, single-joint blending, position-only depth stages,
and cutout discard. None is retained. The opening submits about 100,000 visible
prop instances; parked cars alone account for 7.2 million triangles at tier 3.
The active candidate reduces their distant mesh from 300 to 172 triangles,
keeping near tiers and wreck generation unchanged. Matched visual sheets pass;
the controlled same-scene A/B saves about 3.9 ms (12%) with equal instance counts.
Production live/late-state confirmation and asset validation pass, including
deterministic resets and stable GPU allocations. The car pass is committed.
Next, reassess the remaining GPU cost and native consumer verification; the
whole-spec frame-time target and closeout checks remain open.
The three-band courtyard-wall candidate was rejected in visual review: its
far silhouette lost contrast and definition. Near meshes were unchanged, but
that is insufficient acceptance evidence. No wall change is retained.

Asset validation completed catalog rebaking, runtime byte comparison, icons,
references and transport budgets. Its sole error was the empty directory of the
replaced car bundle; removing that directory and checking all 905 runtime hash
directories against catalog addresses resolves it. Source LFS bytes came from
the local cache through a temporary reader; the asset CLI is unchanged.

The earlier 27 ms “backbone” attribution is withdrawn: its rolling timing window
mixed probe states. Full-map terrain submission is a real architectural issue,
but the corrected measurements do not support prioritizing terrain shadow
chunking for this symptom. Post cost is pixel-bound. Fixed-view comparisons must
also hold local content and view distance constant: seeing farther across a
larger map legitimately admits more geometry.

## Where the frame stands

The battle sizes its render target from CSS dimensions and the selected quality
scale, and the system draws the cursor, so neither a high-density screen nor a slow frame shows up as input lag.
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

### Static batch updates

Presentation and both renderers already retain unchanged static batches. Camera
and world changes invalidate those batches through the existing publication
contract. Do not add another cache on the assumption that every moving unit
forces all props to be repacked; profile the actual invalidation first.

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
