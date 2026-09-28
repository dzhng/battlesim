# Performance

The only gate is **30 FPS average at the default camera, 1920×1080**, measured by the benchmark (the user's floor). Everything else is recorded in the active spec's `frame-cost.md`, not gated; battle-look's rows, the history this page draws on, are in `specs/done/battle-look/frame-cost.md`. The frame has had a lot of headroom (about 8.3 ms p50 at vsync, 2–4 ms of GPU). Keep it that way by measuring every renderer-affecting change, not by guessing.

## Measuring

- **The row:** `bun run --cwd web scene -- benchmark`.
  - It is the production build: the `village-contact` scenario warm-started into heavy contact, flying a fixed six-phase camera tour (strategic, pan, zoom, ground, combined, return).
  - It writes `throwaway/evidence/benchmark/frame-cost-row.md` and a JSON report. Paste the row into `frame-cost.md` with the machine load.
  - Compare rows only within the same scenario and tour version.
- **Hold the GPU lock** so parallel agents don't share the GPU mid-run: `lockf -k throwaway/gpu.lock <command>`. That file is the one convention, and it is not in any script.
- **Machine load is the biggest pitfall.** With other agents running, the same build reads anywhere from 2.1 to 5+ ms of GPU, and p95/p99 spikes to 16.7 ms are contention, not your change. Submit-and-wait lets the GPU clock down too.
- **Measure a feature's cost paired, never as two absolute runs:**
  - use the scene's cost switch, e.g. `GRASS_COST=1`, `FOG_COST=1`, `MODEL_COST=1` or `EFFECT_COST=1`;
  - it interleaves on/off in 1.5 s batches and takes the median of the differences;
  - expect about ±0.3–0.5 ms of noise.
- **Stated tolerance** between runs: frame percentiles within 0.2 ms, GPU mean within 10%. The worst-window p95 is noise, not a comparison number.
- **GPU time is the `timestamp-query` whole-frame total,** a 240-frame rolling mean and p95 read without stalling. There's no per-pass split on Apple; for one pass, use the paired switch.
- **Probes must force a redraw every frame.** The lab draws on demand, so an idle frame just reports vsync.
- **Prove the GPU.** Headless shells silently fall back to SwiftShader, whose timings are void. Record the adapter string with the numbers.

## What has cost what (touchstones, not a ledger)

`frame-cost.md` is the ledger. These show the orders of magnitude:
- **The foundation** (sky, terrain, cascades, post): about 1.2–1.5 ms GPU.
- **Fog:** 0.4–0.7 ms typical and about 1.5 ms worst at 100 units a side. Memory is its real cost, about 1.1 MiB per eye.
- **Grass:** about 0.7 ms mean and 1–1.7 ms at ground zoom, against a 4 ms kill bar. GPU-built clumps per 4 m tile, with no CPU residency, no shadow and no prepass: Apple's hidden-surface removal covers overdraw.
- **Forest:** about 0.9 ms. The 44k backdrop instances beyond the map cost about 0.17 ms.
- **Models:** not measurable at battle scale, thanks to projected-pixel tiers, impostor cards and view culling. All-LOD0 would cost about 1.9 ms more.
- **Effects:** one instanced pass, not measurable.
- **Scars:** one rgba8 texel per metre, uploading only dirty 16×16 tiles. Not measurable.
- **The strategic camera is always the most expensive phase:** the widest shadow reach and a full-frame patchwork.

## Wins worth repeating

- **Casters one tier coarser than the view** (trees): −0.5 ms.
- **Recompute only on change.**
  - Models' and scenery's `prepare` return early on an unchanged detail key.
  - Grass regrows only when the view or the scars change.
  - Fog maps rebuild only for eyes that moved.
  - Scars upload dirty tiles only.
- **Nothing allocates per frame on hot paths:** camera packing, receiver range, cascade scratch and instance packing. One inverse view-projection rebuilt for each of 169 rays cost about 0.4 ms of CPU.
- **Install only what the battle draws:** soldiers' appearances only, not the whole catalog, cut 131 → 77 MiB.
- **Coarser, fewer vertices for overlays that rebuild:** the contact glyph rebuild went from 10.9 ms of CPU to 3.6 ms, and the slow frames disappeared.
- **A static far-chunk buffer drawn in chunk order,** with adjacent chunks merged into one draw range.

## Traps

- **Per-fragment loops over authored rectangles.** Looping 120-odd forest cells in terrain and grass fragments cost about 2.5 ms, so the forest floor stayed one rectangle.
- **Memory grows faster than time.** Textures went 63 → 346 MiB over the port (the MSAA HDR target alone is 63 MiB, shadows 64 MiB); buffers went 37 → 218 MiB, mostly model tiers and variants. Report memory in every row, and question each new full-screen MSAA target: the overlay's own 4× target is 32 MiB at 1080p.
- **Leaf cards and tree impostors were rejected.** They need a discarding prepass and caster, and they shimmer under 4× MSAA. Trees keep an 80-triangle far tier instead.
- **Thinning the horizon to save its 0.17 ms loses the horizon.** Don't.
- **Porting `~/dev/game` wholesale:** its grass quality ladder ran 22–31 ms on Metal, and its JSON-float animation reached 3.3 GB. Borrow ideas through the reuse manifest; don't copy the architecture.
- **Never pass GPU buffers through changing React props.** React 19's development build copies each commit's changed props into a `performance.measure` detail, two levels deep, typed arrays element by element. The overlay's `Float32Array` and the fog world's grids made details of tens of megabytes a commit, and a long battle under `bun run dev` ran out of memory. Hand such data to the viewport through a stable `Feed` (`apps/battle-lab/src/feed.ts`). The scene runner fails any page whose measure details pass 5,000 entries.
- **The overlay's depth is its own 4× target (31.6 MiB at 1080p),** the prepass depth copied before the grass. Grass blades in the overlays' depth test speckled every route and ring.
