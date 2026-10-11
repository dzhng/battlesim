# Native rendering evaluation

This evaluation keeps the browser TypeGPU/WebGPU renderer as the shipped client and records the Godot desktop path as an experimental renderer probe. Godot consumes the browser-owned menu-reel capture, saved-map exports, authored scene catalog, and observed unit publications. It can present the same workload through a real display, but the final images do not meet the browser's visual target, so the native client is not promoted.

## Why the boundary exists

The simulation remains the only authority for rules, fixed ticks, visibility, observations, replay and digests. A native renderer needs a way to inspect that authority without recreating it. The accepted capture-reel seam records the exact Rust publication layout, raw publication words, fog payloads and displayed camera timeline; the reel validates and replays that artifact without advancing a second simulation. The live binding probe uses GDExtension for an in-process experiment, while the capture path remains the measured comparison seam.

The browser and native paths share workload identity, scene keys, camera intent, saved-map placement and catalog ownership. They may use different shaders and batching only while the result stays visually equivalent. That condition is deliberately stricter than “the same counters passed”: the committed browser/native comparisons show a recognizable tank and authored buildings, but missing crop texture, dense vegetation, dust, relief, grounding shadows, material richness and balanced framing. The browser therefore remains the release baseline.

## Principles and invariants

- Rust owns simulation, observations, visibility, replay and digests. Godot observes those outputs and adds no gameplay authority.
- Presentation captures are replay data, not a second wire protocol. Their layout, publication carriers and displayed-frame timeline stay browser-owned; the browser boundary rejects malformed samples, while native replay rechecks packed groups, fog and ground bounds while decoding.
- Saved maps own placement. Native presentation budgets may cull or bound instantiated meshes, but they do not change map admission, coordinates or visibility semantics.
- Authored catalogs own model identity. When a kit path is configured, authored shells own building presentation; configured-but-unresolved templates are omitted and reported as a catalog blocker, while primitive boxes are suppressed whenever kits are active and used when no kits are configured. Units, scenery and grass use catalog lookup paths with explicit primitive fallbacks for missing assets.
- Reports distinguish capture validity, catalog admission, fog consumption, named-cut coverage, display backing and authored-material review. A reviewed failure keeps `comparison_ready=false`.
- Headless Godot frames are lifecycle evidence only. Performance and visual claims require a display-backed run and matched browser controls.
- The synthetic `/benchmark?preset=city-contact` workload remains separate from the menu-reel evaluation.

## Code pointers

- `native/godot-spike/reel.gd` owns the native capture loader, semantic replay, saved-map composition, authored-shell selection, catalog scenery and unit loading, camera playback, cuts and report gates.
- `native/godot-spike/presentation_capture.gd` owns the native-side capture envelope/parser; its replay decoder checks packed groups, fog and ground before semantic publication replay.
- `web/src/battle/benchmark/presentationCapture.ts` owns the browser capture boundary and rejects malformed samples, camera poses and publication carriers before export; `native/godot-spike/presentation_capture.gd` validates the capture envelope and parses layout, while `decode_publication()` checks packed groups, fog and ground before replay.
- `native/godot-spike/reel_lifecycle_test.gd` pins the map, catalog, grass, sky, authored-shell and report-attestation contracts. `web/tests/presentationCapture.test.ts` pins the browser JSON boundary.
- `fixtures/menu-backdrop.json`, `fixtures/maps/`, `fixtures/catalog.json`, `fixtures/units/menu/units.json` and `assets/catalog.json` remain the content owners. The Godot project is a consumer of those sources.
- `specs/done/native-rendering/evidence-manifest.json` records the workload fingerprint, capture hashes, viewport and final ignored reports. The ignored reports and full cut sets remain under `throwaway/` because they are run artifacts rather than source fixtures.

## Dead ends retained as rationale

The synthetic cube field was useful for proving that a camera and report could run, but it hid every asset and composition defect and was removed from the authored path. Headless or dummy-renderer FPS was rejected as a comparison number because it has no displayed frame. Sparse fixed grass samples were replaced by deterministic samples derived from saved forest polygons plus a bounded target-near meadow grid. Procedural sky colors alone produced a flat gradient and were replaced by a deterministic visible cloud shader. Drawing primitive building boxes beside authored shells duplicated and obscured the real buildings, so shells now own that presentation when kits are configured.

## Visual provenance

The browser report recorded a 1280×800 CSS viewport at DPR 2; the committed browser control plates were cropped to 1280×720, matching the native cut viewport. The native images are representative display-backed Godot cuts from the final authored run. Together they preserve the visual standard and the accepted failure record; a browser `paris-corner` plate was not retained:

- [Browser market-town control](visualizations/browser-market-town.png) — broad field, crop texture, dust and atmospheric sky set the target for the native pass.
- [Browser close control](visualizations/browser-market-town-close.png) — close vehicle and infantry framing set the readability target.
- [Native market-town result](visualizations/native-market-town.png) — authored units and buildings render, but terrain, vegetation and grounding remain sparse.
- [Native city-horizon result](visualizations/native-city-horizon.png) — authored city shells render, but materials and composition remain behind the browser control.

## Final verdict

The display-backed evidence is complete enough to make the product decision: both scene captures are valid, the full named cut set was saved, all camera-visible instantiated building rows resolved through the selected kits with an empty unresolved-template report, and catalog lookup paths plus primitive fallbacks were exercised for observed units and scenery. The representative images show the remaining visual gaps, and the review attestation is recorded in `throwaway/native/authored-report-reviewed.json`; it does not change the visual result. Godot remains experimental, the browser remains shipped, and a future native promotion needs a new spec for materials, terrain/grass density, effects, grounding, atmosphere and framing rather than treating this probe as parity.
