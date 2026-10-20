---
name: renderer
description: Architecture rules for GPU rendering with TypeGPU/WebGPU. Use when adding or changing render passes, shaders, frame orchestration, GPU resources, culling, terrain, shadows, or visual performance work.
---

# GPU renderer

The renderer is TypeGPU over WebGPU. It consumes plain frame inputs from the
application and owns pass ordering, GPU resources, and presentation of those
inputs. It never reaches into simulation or app state.

## Workflow

1. Find the existing owner for the frame, camera, depth convention, targets,
   resource registry, culling, and timing. Extend that owner instead of adding
   a parallel path.
2. State the work bound before changing code. For a fixed camera and render
   size, screen-space passes must be bounded by pixels; world geometry and
   shadow work must be bounded by the camera and fitted shadow regions, not by
   total map extent. A huge map must not submit a map-sized terrain mesh or
   off-view shadow casters for a fixed view.
3. Measure the current frame with an interleaved A/B on the real GPU. Isolate
   terrain, shadows, models, scenery, fog, post, and overlays where possible;
   whole-frame timestamps overlap on tile GPUs, so do not add individual deltas
   as if they were independent.
4. Choose the pass by semantics: depth prepass for opaque geometry, lit colour
   after depth, translucency after opaque, screen-space work on resolved
   targets, and overlays/UI last. Keep one frame function as the orchestrator.
5. Update resources at their natural frequency. Allocate through the registry,
   use scoped replacement for resize or pipeline changes, and prove repeated
   rebuild/reset returns allocation counts and bytes to baseline.
6. Verify the narrowest real scene on hardware, then run the owning package
   checks. Treat validation warnings, device errors, and visual regressions as
   failures.

## Architectural rules

- **One owner per contract.** Camera projection, reverse-Z depth, frame targets,
  culling bounds, lighting, and resource lifetime each have one authoritative
  owner shared by all passes and clients.
- **Bound submitted work, not only rasterized pixels.** Frustum clipping does
  not make vertices free. Tile terrain and large static populations; select
  shadow casters against the fitted receiver; keep per-frame lists bounded by
  visible/casting regions.
- **Keep map size out of fixed-view cost.** Terrain, props, and scenery may be
  stored map-wide, but draw submission and shadow casting must use spatial
  chunks or equivalent bounded selection. Post-processing and composites should
  scale with target dimensions only.
- **Preserve depth and lighting contracts.** Reverse-Z comparison, clear value,
  format, multisample count, and shared vertex positioning change together.
  World materials read the shared lighting/shadow function; do not introduce
  per-layer lighting shortcuts.
- **Respect GPU lifetime.** Every buffer and texture has a registry owner.
  Async rebuilds install a complete replacement, release overtaken builds, and
  remain safe after disposal.
- **Keep output authority in presentation.** The renderer draws the records it
  receives; optimizations may cull or change detail only through the existing
  presentation/quality contract and must preserve deterministic output.
- **Judge pixels as well as counters.** Pair draw/GPU statistics with a real
  screenshot or crop that proves the subject, framing, fog, depth, and shadows
  remain correct. Use an unprimed visual review for player-facing changes.

## Performance acceptance

Record the trigger, amplification, owner, A/B evidence, and remaining bound in
the active performance spec before promoting an optimization. A candidate is
 not accepted because one pass became cheaper if the frame, image, or resource
 contract regressed. Keep rejected experiments out of production code and
 document only the durable reason for rejecting them.
