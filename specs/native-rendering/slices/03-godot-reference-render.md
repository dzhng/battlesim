# Slice 03 — Render the reference reel in Godot

## Contract unlocked

Godot renders the same frozen full menu-reel workload as the browser control by consuming `battle-presentation-capture/v1`. The displayed-frame report includes average FPS, 1% low, absolute minimum, maximum, raw intervals and chart, plus separate simulation, transfer, CPU submission, GPU, presentation, startup and shader-compilation timings.

## Seam and ownership

Use only the binding selected by slice 02. Godot consumes shared observations, camera intent, public geometry and validated scene assets. It does not run simulation rules, navigation, visibility or fog authority. Preserve project coordinate, camera and depth conventions; batch instances deliberately and record unavoidable material, lighting, shadow, fog or post-processing differences.

Match resolution, quality intent, warm-up, scene order, camera shots and presentation path. Offscreen/readback timings do not substitute for displayed-frame timings.

## Runnable artifact

The current `native/godot-spike` project includes `reel.tscn`, a camera/workload probe that reads the browser-owned `fixtures/menu-backdrop.json` and plays every authored scene and shot. Its report identifies the scene identities and explicitly marks `comparison_ready=false` because the world is still a synthetic proxy. Slice 03 remains open: extend this probe to consume `battle-presentation-capture/v1`, decode side publications, load authored map assets, and save named cuts before making any performance claim. Godot remains labelled experimental.

## Verification

Run Godot smoke/report checks, then repeat browser control and Godot sequentially under the GPU lock. Compare the same crops at every named cut with [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md); the slice variable is scene identity, camera framing, visible population and fog/veil state. Pixel-level material and post-processing differences are out of scope unless they change the agreed visual target. Finish with [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) and show evidence with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md). The visual checkpoint is non-blocking; record corrections or proceed after the response window.

## Delegated choices

Godot scene organization, import settings, batching strategy and chart drawing are delegated. Scene identity, authority ownership, timing schema and comparison crops are fixed.

## Must stay green

Slices 01–02, browser menu and benchmark scenes, Rust replay/digest tests and browser checks affected by asset loading.
