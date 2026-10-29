# Slice 04 — production browser feature parity

**Status:** complete. Production checkpoint `69a01c6e4` is reviewed and pushed;
retained diagnostic gates also pass after TypeGPU deletion.
Shared paint/x-ray, high-air, canonical lab/proxy models, impostors, inspector
modes, queries, production viewport and sheets use the common runtime.

The native replay lacks player/browser surfaces. Preserve them through shared
Rust GPU owners before replacing the production renderer. JavaScript continues
to own input, DOM/HUD, camera control and external geometry construction.

Implement and verify independent passes for:

- Lit ground paint, marching paint and pointer marks; distinct opaque/translucent
  display overlays, depth behavior, information colors and glow sizing.
- X-ray and hidden-fragment coverage, selected/own/ally routing and placement ghosts.
- Separate high-air output, premultiplied page composition and correct grading.
- Workbench/proxy/lab models, authored poses, surfaces and impostor behavior.
- Quality controls, including shadow size, grass density and feature toggles.
- Diagnostics, debug/channel/surface views, readback, fog/grass probes and exact
  submitted geometry counters. Do not implement unsupported methods with fake data.

At each pass, drive the existing practical player/lab entrypoint with its real
inputs, pin the behavioral contract first, inspect before/after pictures, obtain
an unprimed critique and preserve approved visual checks. Reslice a feature when
its remaining uncertainty prevents an independently reviewable implementation.

Then replace the browser interface: the shell receives canvas(es) and plain
inputs, not a JavaScript GPU device or `GPUTextureView`. Rust owns GPU handles,
targets, lifetime, drawing and capture. Keep the public application behavior and
loading/failure/input/audio contracts while deleting the old GPU interface.

Finish with the final three-target migration/deletion gate in slice 05.


## Current implementation evidence

The browser capture includes real paint/pointer/opaque/translucent streams,
selection x-ray, articulated tooling poses and placement ghosts. Its final/world
source image differs from TypeGPU by mean RGB `0.0000014467592592592592`; the six
other inspector views match exactly. Mode exit restores the full image. Both
canvas outputs survive resize and device recreation in the matched run. The
captured street has no high-air pixels; actual aircraft/card high-air rendering
is covered by native GPU tests and still needs a nonempty browser composition gate.

Impostor baking uses the actual exported MRT pipeline. Albedo, normal, mask,
metadata and hash match TypeGPU exactly. Real GPU tests cover relit/tinted cards,
card-only high-air routing, suppression and owned snapshots after catalog drop.
The coarse-mesh fallback is deleted. Startup admission borrows presentation
synchronously, then awaits GPU asset preparation without a presentation borrow.

Lab model admission uses authored bundles and canonical pose/batch owners;
replacement removes only the tooling suffix of appearance rows. Ground texture,
road, material channel/surface and display controls reside in shared owners.
Fog/grass/readback queries use explicit owned GPU snapshots. Browser readback
preserves the original frame while another view renders before mapping completes;
its shared fog-shape probe returns the required ordered GPU samples.

Raw evidence and the remaining native numerical investigation live in
[research](../research.md). The normal application now receives canvas/pixel APIs from the shared Rust
runtime. The TypeGPU control is frozen as scratch evidence; its runtime and dependency
are deleted by slice 05.

## Acceptance evidence

Production viewport and offscreen sheets use canvas/pixel APIs and one page GPU.
Latest-generation appearance, proxy and facade/workbench consumers pass the
production checkpoint. Nonempty high-air, impostors, quality rebuilding,
material controls, exact submitted counts and grass/fog probes are implemented.
The named terrain depth-policy correction remains unchanged.

Deletion retains the former ownership controls through ordinary BrowserScene:
queued snapshots preserve their submitted image while a different view renders;
resize, surface reconfiguration and actual device recreation preserve pixels;
recovered fog queries retain both seen and unseen points. Ghost alpha, depth and
fog rules pass for skinned, articulated and static authored models. All four
terrain material helpers preserve distance and bit-exact filter gates, and
release every observed buffer/texture wrapper after explicit collection.

The full fog journey also passes its unchanged performance.measure guard after
live observation props move through the existing stable feed boundary. Detailed
raw evidence and instrumentation limits remain in research; repository-wide
closeout belongs to slice 05.
