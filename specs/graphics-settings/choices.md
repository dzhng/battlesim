# Final choices

- **Persist the player preference under `battle.graphics`.** This follows the existing browser-owned sound store and keeps renderer code independent from storage. Confidence: high; required by the plan.
- **Treat malformed or unknown records as first-launch Auto-detect.** Hard cutover was explicitly chosen; no migration path is retained. Confidence: high; user decision.
- **Use a shared `GraphicsSettingsControls` in both menus.** This gives one radio-group and status contract rather than two drifting settings implementations. Confidence: high; user accepted the shared layout.
- **Apply a selected tier by rebuilding the viewport generation.** Existing renderer ownership already centralizes GPU lifetime there; the replacement is built before the previous generation is disposed. Confidence: high; required by the renderer contract.
- **Keep bloom, cast lights, and authored mood fields fixed.** The quality profile only changes cost controls and conservative grass. Confidence: high; user constraint.
- **Use a bounded requestAnimationFrame probe for first-launch auto-detect.** This is a provisional implementation of the canonical workload integration; the full per-tier reel sweep remains closeout work. Confidence: medium; chosen to keep startup bounded.
- **Keep a valid but below-target adapter on Low.** Fatal handling is reserved for missing WebGPU or unusable measurements; a slow but measurable device should remain playable. Confidence: medium; implementation judgment, reversible through the resolver.
- **Fatal resource errors remain visible on the menu.** The old menu swallowing behavior was changed because the user required WebGPU and detection failures to stop startup. Confidence: high; user decision.
