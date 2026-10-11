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
