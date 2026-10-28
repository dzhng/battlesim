# Slice 04 — production browser feature parity

**Status:** active. Shared paint/x-ray, high-air, canonical lab models, actual
impostors, inspector modes and core GPU queries are implemented. Browser
viewport/workbench integration and remaining feature gates are still open.

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
[research](../research.md). This does not complete the slice: the normal
application still owns its TypeGPU canvas interface.

## Remaining gates

- Replace production viewport and offscreen sheet GPU-handle interfaces with
  canvas/pixel APIs; preserve loading, failures, recovery and latest-generation
  appearance replacement.
- Admit box/marker proxies through ordinary model assets/batches; preserve
  infantry's pick-only behavior. Exercise the real workbench/facade consumers.
- Exercise nonempty browser high-air composition, configured review bakes,
  live material/quality toggles, real fog/grass probes and exact submitted counts.
- Complete quality rebuilding, including shadow size and grass density, and
  preserve controls across rebuilds.
- Preserve the named terrain depth-policy correction: the complete tiny native
  delta is isolated to removing the obsolete `always` depth workaround.
- Commit/push the reviewed core checkpoint, then continue production consumer
  migration. Independent review's fog recovery, lab cards and paint clear
  findings are fixed; regression evidence is in research.
