# Slice 02 — shared production frame checkpoint

**Status:** complete 2026-10-28. Final migration acceptance belongs to slices
[03](03-runtime.md), [04](04-player-features.md) and [05](05-migration.md).

Native and browser now invoke one concrete `FrameRenderer` over the actual
shared resource owners. The callback stage adapters, browser ground/environment/
fog approximations and native pass wrappers are deleted. Presentation continues
to own poses, visibility, tiers, casters and held static batches.

The complete captured street matches the TypeGPU browser control essentially
byte for byte; all six native plates are byte-identical before/after extraction.
Unprimed critiques and local crops support continuity. The default eleven-row
scene and actual device-loss recreation pass. The recovery regression preserves
complete frame pixels, including player marks, without manual scene replay.
[Research](../research.md) owns measurements, evidence and their limits.

This checkpoint establishes shared drawing, not the final architecture. Native
and browser still compose resources and prepare records separately. Browser
transport copies canonical presentation state through another WASM memory.
Effect-atlas creation/mips and some browser features also remain outside the
common owner. These mechanisms must disappear into the shared runtime before
TypeGPU deletion can count as a complete migration.

A scratch experiment established that existing presentation, renderer and sound
bindings can share one browser module and Rust can borrow the presentation
synchronously. Slice 03 turns that packaging proof into the actual GPU path.
