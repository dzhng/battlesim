# Slice 04 — production browser feature parity

**Status:** pending. Runtime ownership depends on slice 03; independent feature
investigation and implementation may proceed against its canonical interfaces.

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
