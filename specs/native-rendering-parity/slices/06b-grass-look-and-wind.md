# 06b — Grass: look and wind

**Contract:** native grass reads like the browser's: blade shape, colour mix, crop rows, dryness, near/far LOD shrink, and wind sway driven by the recorded clock.

**Seam:**
- Native `grass/grass.gdshader` ports the blade vertex and fragment formulas from `grassPass.ts`, reading `grassTuning` and the kinds and shapes from the manifest.
- Wind uses the frame record's `simTime`, so the sway phase matches the browser's frame.

**Runnable artifact:** a close and a broad plate pair over meadow and crop.

**Visual variable:** the grass's overall impression.

**Focused gates:**
- Counts from 06a stay within about 10%.
- Run `compare-screenshots`, then unprimed `screenshot-critique` as the last check, then `preview-shots` (non-blocking).

**Out of scope:** scars and trampling (phase 2), and trees as authored models (phase 2).

**Delegated:** the blade mesh topology, provided its silhouette and LOD match.
