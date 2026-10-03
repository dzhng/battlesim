> Frozen historical evidence. Original identities and rejected arms remain scoped to their source; current completion and user-authorized deferrals are recorded in [completion evidence](../../evidence.md).

# Stopping checkpoint (commit `9a88280`)

Production contracts and reviewed spec work were gathered for main with the full check gate green (394 simulation tests, 13 saved-map resolver tests, 13 compiler tests, 516 web tests) and all 412 browser checks passing. The pure resolver is integrated; catalogue cutover and complete preparation admission remained pending. The [ground review](visual-review.md) holds the scoped parity and visual findings.

The cleanup scene's exact byte assertion had failed twice by 2688 bytes. The cause was view-dependent, not a leak: drawing the final tick at the starting camera before the camera jump grows two scenery near-tier buffers from 3072/3072 to 3936/4896 bytes. The harness now waits for the existing `presented` helper before moving the camera, so every cycle draws the same view; no assertion, tolerance or renderer policy changed.

Raw evidence (gate logs, allocation probes, capture manifests): tag `city-maps-evidence-2026-09-30`.
