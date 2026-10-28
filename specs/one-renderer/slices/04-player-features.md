# Slice 04 — production browser feature parity

**Status:** active. Isolated x-ray GPU owner/regression implemented; production
depth/overlay integration is next. Runtime ownership depends on slice 03; independent feature
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


Current owner gaps (not completion waivers): Rust still draws high-air models
into the world, substitutes far posed meshes for actual impostor cards, and lacks
lab-model/proxy admission and material/surface/debug controls. Ground paint and
pointer geometry must bind through shared fog/material policy, not be flattened
into display overlays. The x-ray test currently adds depth-copy usage locally;
production integration must add it to the actual targets and remove that local
substitution so the GPU regression covers the real allocation contract.
