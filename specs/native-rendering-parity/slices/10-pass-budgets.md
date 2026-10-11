# 10 — Per-pass GPU budgets

**Contract:** each native pass costs no more GPU time than the browser's equivalent pass on the same frame. The per-pass table from 08 is green, row by row.

**Seam:** the passes as both clients label them: shadow cascades, terrain, grass build, grass draw, sky, haze, post (bloom, grade, AgX). Native looks first at how it reads the browser's tables:
- the terrain shader reassembles each word from four RGBA8 texels; candidates are a storage buffer in a compute-built ground cache, or a packed format Godot accepts;
- the grass build reads the same tables;
- grass shadow casters.

The browser's own pass costs are the target. If a browser pass is itself slow, say so, but don't fix the browser here.

**Also from 08:** presenting to a shown window adds about 17 ms a frame (26 → 43 ms on 300 market-town records). That is the shipped client's cost. Find what presentation does (swapchain image count, vsync mode, MSAA resolve into the drawable) and bring it to a few ms.

**Runnable artifact:** the per-pass table, browser beside native, before and after, in `throwaway/perf/passes/`.

**Focused gates:**
- Every pass is at or below the browser's, as the median of three runs, plugged in.
- Plates within the noise floor.
- Native tests green.
- Any change to how tables are read keeps the 05a class-agreement and 06a hash and count tests green. Restore the class-view check from history (`48c3ab7a`) if the terrain table path changes.

**Visual variable:** none.

**Delegated:** the technique per pass, provided the picture is unchanged.

## Findings (2026-10-11)

Editor (debug) build, plugged in, M5 Pro, 1280×720, uncapped, 60 warm-up frames, three runs a figure. Evidence in `throwaway/perf/passes/` (`table.md` for the passes, `cmp-before/` and `cmp-after/` for the reels).

**A new control.** The merge of main (`4ef792d8c`) changed the tank and soldier models, so the browser control was re-run (`throwaway/native-parity/2026-10-11T08-56-34-958Z`, both scenes) and native's staging re-imported from scratch. Every number below is against it, and slice 12 uses it too. Its frame records differ from the 10 October run, so the 1000–1299 window now holds a different shot (714 draws, 7.6 M primitives, not 09's 410 and 4.2 M).

**Whole reel, both scenes, three runs** (median run; each run's p95 and p99 in brackets):

| | market-town p50 / p95 / p99 / max | paris-corner p50 / p95 / p99 / max |
| --- | --- | --- |
| Browser | 16.59 / 25.07 / 26.09 / 65.88 (p95 25.00–25.07, p99 25.81–26.09) | 8.43 / 17.21 / 24.29 / 42.05 (17.00–17.45, 18.17–24.34) |
| Native before (`92edf9480`, fresh import) | 9.93 / 24.42 / 26.34 / 34.62 (23.96–25.11, 25.81–27.29) | 8.49 / 10.55 / 11.66 / 21.63 (10.35–10.71, 11.30–15.33) |
| Native after (`2c04e93fe`) | 8.33 / 14.79 / 16.03 / 17.85 (14.07–18.23, 15.44–20.22) | 8.10 / 9.28 / 9.60 / 12.72 (8.71–9.28, 9.12–9.61) |

Gate: native p95 and p99 are below the browser's in every run, both scenes. Before, market-town's p99 was above it (26.34 against 26.09).

**The slow tail was the kit shells.** 09 drew all four tiers of every shell, near or far, so a wide city view drew each far block's 35 k-triangle tier 0 behind its 1.7 k tier 3. The browser tiers a building as one: its shell and its facade modules at the chunk's tier (`buildingLayer.ts`, `presentation.buildings.lod_px_per_m`). Native places no modules, and a shell's walls are cut open for them at tiers 0 and 1 (`OPEN_TIERS`, `kit.py`). So an open tier now draws over every coarser tier, which fills what the modules would, and tiers 2 and 3 draw alone. Primitives fell from 16.2 to 8.5 M a frame on market-town (peak 33.0 to 14.0 M).
- Two narrower rules were tried and dropped on the plates. An open tier over tier 2 alone left Paris dormers hollow, because the dormer is in tier 1. Over tiers 1 and 2 cost a near balcony its front face, which is in tier 3; the unprimed critique found that one.
- Near plates are unchanged within the noise floor (paris-corner: at most 29 levels on 612 pixels of one plate). Far buildings changed on market-town, with the same distance to the browser (mean 0.0642 → 0.0643, no plate further by more than 0.001).

**Far props: no change, because the browser has no far cut either.** Props are culled to the view and draw their coarsest tier out to the horizon (`modelDetail.ts`). Impostor cards are only for skinned soldiers and corpses. Native follows the same rule, and also drops map rows farther than 1 km from every frame target (legacy layer). With the shells fixed, props are not in the tail.

**Per pass** (market-town records 1000–1099 drawn 40 times over, against the browser's same stretch of reel; Metal System Trace, ms of GPU per frame):

| Pass | Browser | Native before | Native after |
| --- | --- | --- | --- |
| Whole frame, GPU busy | 9.78 | 6.31 | 4.43 |
| Shadow cascades | 2.43 | 0.34 | 0.33 |
| Terrain | in its world pass | 2.23 | 0.79 |
| Grass build | 1.12 | 0.32 | 0.21 |
| Grass draw | in its world pass | ~0 | 0.06 |
| Sky | 0.14 | 0.43 | 0.03 |
| Haze | in its world pass, per fragment | 1.52 | 0.01 |
| Post (bloom, grade, AgX) | 0.68 | ~0 | 0.17 |
| Browser world colour + depth prepass | 6.14 + 1.32 | | |

- **How it was measured.** Browser rows are its labelled encoders. Dawn passes WebGPU labels to Metal under `--enable-dawn-features=use_user_defined_labels_in_backend`, a scratch flag for the trace only. Native rows are the GPU busy time saved by switching the pass off, because Godot's labels don't reach Metal. A changing record set skewed the first native traces, so the probe loops 100 records.
- **Precision.** Whole-frame and sky, haze and browser rows are medians of three traces. Other native rows are one trace each, with about ±0.3 ms of trace noise. Ablation rows overlap, because busy time is a union.
- **Untraced.** The browser's untraced GPU time is 10.0 ms (timestamp queries), close to its traced 9.8. Native has no Metal timestamps; its traced frames are paced at 8.33 ms with the GPU idle about 45% of each.
- **Sky and haze were over the browser** and are fixed. Both integrated the 32-step physical sky per pixel, the haze twice. The browser bakes that sky once into a 384×192 equirectangular LUT and samples it in its background, environment light and aerial term. Native now bakes the same lookup (`environment/sky_lut.gd`) and samples it in the same three places. Whole-frame GPU busy fell 6.31 → 4.43 ms. Plates move at most 4 levels (linear filtering of the LUT), neither toward nor away from the browser: the changed pixels' mean distance to the control went 10.608 → 10.616 (market-town) and 21.888 → 21.871 (paris-corner).
- **No pass is over the browser's.** Native's terrain, grass draw, models and haze together (about 1.2 ms) are well under the browser's world pass and prepass (7.5 ms), which hold the same work.

**Presentation is no longer a cost.** Measured on the 1000–1299 window, eleven runs, with each run's on-screen state verified from the window server, since neither the project's minimised start nor `--windowed` decides it: macOS restores whatever state it last saved.
- On screen (6 runs): p50 8.25–8.28 ms, p95 14.0–14.4.
- Off screen (5 runs): p50 8.10–8.21 ms, p95 14.4–14.5.
- So presenting costs about 0.1 ms. 08's +17 ms was measured with the GPU saturated by 26 ms frames of the legacy layer's work; with frames well under the display interval, presenting overlaps.
- Swapchain image count (2, 4) and frame queue size (1, 3) moved p95 by under 1 ms, within the spread, so nothing changed.
- The MSAA resolve does not go into the drawable, by Godot's design rather than measured here: the effects read Godot's resolved colour (`access_resolved_color`), and the tone-mapped 1280×720 target is blitted to the drawable.

**For slice 12.**
- Native's "uncapped" frames are paced at the display's 120 Hz even off screen: medians sit at 8.33 ms, and paris-corner averages exactly 120.0 FPS. Both clients are therefore display-paced, and the comparison holds while frames exceed 8.3 ms; below it, compare GPU time from traces.
- The report's `window` field reads what the window was, which runs don't control: a bench meant to be minimised can be on screen.
