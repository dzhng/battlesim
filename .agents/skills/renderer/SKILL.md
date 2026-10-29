---
name: renderer
description: Architecture rules for the shared native/browser GPU renderer. Use when adding or changing render passes, shaders, frame orchestration, GPU resources, culling, terrain, shadows, or visual performance work.
---

# GPU renderer

Use the shared Rust `wgpu` runtime for native and browser WASM. Keep platform
shells to device/surface admission and transport. Draw canonical presentation
inputs without querying simulation or app state. Check each change against
these rules before writing it.

- **Fixed-view cost never grows with world size.** For a fixed camera and
  render size, submitted geometry, shadow casters, and per-frame lists are
  bounded by what the view can see or cast into, and screen-space work by
  pixels. Store the world map-wide; select from it spatially. Frustum clipping
  does not make vertices free. Compare equal local content and finite view
  distance: admitting more world changes the workload.
- **Detail follows projected size.** Correct culling can still admit thousands
  of tiny meshes. Bound each visible instance’s geometry by its screen footprint
  as well as bounding the selected population.
- **One owner per contract.** Frame orchestration, camera, depth convention,
  targets, culling, lighting, timing, and resource lifetime each have one
  owner every pass shares. Extend it; a second copy drifts.
- **Shared contracts change together.** Depth and lighting are engine-wide:
  every pass reads the same convention and every material the same shading
  path. A per-pass exception is a future visual bug.
- **No GPU resource without a lifetime owner.** Scene and resource owners
  retain their allocations, rebuilds swap in whole, and repeated
  resize/rebuild/reset returns to baseline.
- **Pixels prove correctness, not counters.** An optimization is accepted
  only with an interleaved A/B on the real GPU and an image that shows the
  subject still drawn correctly.
