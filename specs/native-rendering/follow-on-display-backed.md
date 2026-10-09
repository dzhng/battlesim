# Follow-on — display-backed authored comparison

This is the next experiment after the native-rendering evaluation. It is deliberately separate because the current Godot evidence is headless and uses bounded primitives; its frame rate cannot be compared with the browser's foreground run.

## Contract

A foreground Godot run consumes the existing `battle-presentation-capture/v1` files, the saved-map exports, and the authored scene catalog. It renders terrain, roads, props, buildings, fog/visibility and decoded observed units at the captured camera timeline. The Rust simulation and layout remain the only authority for observations and visibility.

Both clients produce the same scene-keyed cuts at the same 1280×720 viewport and quality intent. Reports include displayed-frame intervals, startup, capture transfer/decode, CPU submission, GPU, presentation, shader compilation and memory fields; unavailable fields remain explicitly null. A run is comparison-ready only when all scene captures and named cuts exist and the report has no incomplete-world blocker.

## Gates

- Hydrate the reviewed China and Paris authored kits and load the full catalog without a placement cap in a foreground Godot run.
- Add the map's terrain/material and fog layers while preserving the captured camera poses.
- Capture every named shot for both menu scenes and retain the browser control cuts beside them.
- Compare matching crops with `compare-screenshots`, inspect the complete set with `preview-shots`, and obtain an unprimed `screenshot-critique`.
- Repeat browser and Godot runs sequentially under the GPU lock; aggregate distributions rather than selecting a peak run.
- Advance Godot only if visual composition is readable and maintenance/platform evidence justifies the extra client. Otherwise keep it experimental and archive the candidate failure.

The current native-rendering spec remains the record of the first evaluation; this follow-on owns only the missing display-backed evidence.
