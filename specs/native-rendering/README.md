# Native rendering and Godot evaluation

**Status:** handed off mid-implementation; browser route smoke and native parity pass, with a browser frozen-capture exporter, Godot authored-reel camera playback, capture decoder and an optional authored-scene loader. Authored asset rendering and matched performance remain unbuilt.
**Updated:** 2026-10-09.

## Next Agent Prompt

Next session: run the real browser graphics test and place its per-scene `presentationCaptureJson` artifacts in `GODOT_PRESENTATION_CAPTURE_DIR`, replace the single city kit with the full authored menu-world composition, and capture named cuts in a foreground Godot run. Keep the interactive GDExtension seam and the Rust simulation as authorities. Do not claim comparison-ready or performance evidence until the Godot report consumes real browser publications and named browser/Godot cuts are visually reviewed. Keep `/benchmark?preset=city-contact` unchanged.

Evidence: focused client/reel/report/capture tests, TypeScript, production build, route smoke, Godot decoder test and a foreground reel run with a hydrated city kit placed at both map populations. The graphics-test report now carries one validated `battle-presentation-capture/v1` JSON artifact per menu scene, copied from the publication buffer before credit return. The Godot reel applies the matching scene's decoded camera samples when a capture is supplied and reports how many scene captures and map placements were consumed; `comparison_ready=false` remains because the probe still caps repeated kit placement and does not render terrain, props or observed units. Headless Godot uses the dummy renderer, so its `cuts_saved=0` is lifecycle evidence only; the foreground run produced named PNG cuts, which show the placement path but are not matched visual evidence. A real browser run completed both menu scenes (156 seconds) and emitted the full report; the shared preview was background-throttled to about 1 Hz, so that run is lifecycle evidence only, not FPS evidence. Native replay/side-filter parity and IPC measurement pass. Godot's editor scan still reports the expected missing local extension dylib when the dylib is not built. Authored Godot rendering, visual comparison and performance verdict remain open.
The attempted headless Playwright capture did not produce a report within its timeout, so no browser capture files were promoted from that attempt.

The first foreground cut probe produced 24 PNGs and loaded the authored GLB, but its shots visibly show one centered house against the clear background (and the opening fade is black). This is a renderer-path proof, not a matched menu-world result; retain the cuts as evidence only until the full map composition is loaded.

## Goal

Determine whether a Godot desktop client is worth maintaining alongside the browser TypeGPU/WebGPU client, while preserving the Rust simulation as the one authority. The first evidence run targets the current Mac and keeps the design platform-neutral. The comparison uses the full main-menu battle reel, exposed as a graphics test in Settings for the client currently running.

The graphics test reports average FPS, 1% low, absolute minimum, maximum, raw frame intervals and a frame-time chart. It also records simulation and observation transfer, CPU submission, GPU execution, presentation, startup and shader-compilation costs separately. There is no hard performance threshold; the verdict weighs performance with visual target, startup, platform reach and maintenance cost.

## Fixed decisions

- Browser and Godot share simulation outcomes, observations, scene identities, camera intent and quality intent. Rendering implementations may differ where the result remains visually equivalent; pixel parity is not required.
- The browser remains the shipped baseline. Godot stays experimental until broad gameplay and visual-target parity is demonstrated.
- The first comparison is Godot versus the current browser renderer. Native JavaScript WebGPU and Rust `wgpu` are follow-up probes only if the evidence needs them.
- The full menu reel is the reference workload. Its menu maps, encounters, seeds, warm-up, catalog and camera shots are frozen by identity. Benchmark playback is independent of reduced-motion preferences and the live menu's simulation-clock pacing.
- The existing synthetic `/benchmark?preset=city-contact` remains a separate diagnostic and regression workload.
- The first Rust-to-Godot binding slice is an architecture spike. Candidate seams include direct native/GDExtension, offline replay/presentation capture, and a process boundary. No seam is promoted until it passes the same authority, lifecycle and measurement checks.

## Ownership and firewalls

- Rust simulation and contracts own rules, fixed ticks, visibility, observations, replay and digests. Godot must not add gameplay physics, navigation or fog authority.
- The browser simulation adapter owns borrowed publication copying, credits, decode and lifetime. Native probes must preserve the same bulk-transfer and teardown semantics without copying packed layouts into a second authority.
- `apps/battle-lab/src/menuReel.ts` and `fixtures/menu-backdrop.json` own the menu scene identities. The benchmark manifest may reference them, but must not create a second set of scenes.
- `packages/renderer-core` owns camera and depth conventions; the battle renderer owns observed-feed composition. No general cross-engine renderer abstraction is introduced before the spikes identify a seam worth sharing.
- Temporary probes stay in ignored evidence or a clearly isolated native experiment. Rejected bridges are deleted when their evidence is archived.

## Slice graph

1. [Baseline and graphics test](slices/01-baseline-and-graphics-test.md) — freeze the menu-reel workload, define the shared report and add the browser Settings test.
2. [Binding architecture spike](slices/02-binding-architecture-spike.md) — compare Rust-to-Godot seams with live authority and replay/digest checks.
3. [Matched Godot renderer](slices/03-godot-reference-render.md) — render the same captured workload in Godot and produce a comparable displayed-frame report.
4. [Verdict and reslice](slices/04-verdict-and-next-plan.md) — repeat runs, compare equivalent outputs and write the focused follow-on plan.

Native JavaScript WebGPU and Rust `wgpu` remain optional follow-up probes, owned by slice 04 if Godot's evidence exposes a host question.

## Verification map

Every visual slice must use [compare-screenshots](../../.agents/skills/compare-screenshots/SKILL.md) against the frozen browser reference and finish with an unprimed [screenshot-critique](../../.agents/skills/screenshot-critique/SKILL.md). Use [preview-shots](../../.agents/skills/preview-shots/SKILL.md) to show the evidence. Visual review is non-blocking: open the shots for roughly five minutes, then decide from the evidence, record the rationale and close the shots if no response arrives.

At closeout, run the full repository checks once, as required by `AGENTS.md`. During implementation use the narrowest relevant browser, Rust and native probe checks. Serialize GPU runs with the repository's GPU lock.

## Research to recheck in slice 02

Use primary documentation and pin exact versions in the evidence manifest:

- [Godot renderer architecture](https://docs.godotengine.org/en/stable/tutorials/rendering/renderers.html)
- [Godot GDExtension](https://docs.godotengine.org/en/stable/tutorials/scripting/gdextension/index.html)
- [godot-rust](https://godot-rust.github.io/book/)
- [Godot benchmark suite](https://github.com/godotengine/godot-benchmarks)
- [wgpu](https://github.com/gfx-rs/wgpu) and [wgpu-native](https://github.com/gfx-rs/wgpu-native)
- [TypeGPU](https://docs.swmansion.com/TypeGPU/)

External measurements are context only; the frozen Mac runs are the evidence.

## Global TODO

- [~] Add displayed-frame rate summary fields and tests — slice 01 (landed).
- [ ] Freeze complete menu-reel identity, canonical capture and browser control — slice 01.
- [~] Add Settings graphics test and report chart — slice 01 (route exists; lifecycle, report identity and visual verification remain).
- [~] Verify offline native seam and Godot headless toolchain — slice 02 (offline probe and live extension smoke load; real scenario transfer/parity remains).
- [x] Spike binding candidates and select a seam — slice 02 (GDExtension for interactive authority; presentation capture for renderer comparison; IPC retained as measured fallback).
- [~] Render the full menu reel in Godot — slice 03 (camera/workload probe and capture decoder added; real publication/assets still open).
- [ ] Compare repeated distributions and reslice the chosen direction — slice 04.

## Human review surface

The human reviews the frozen menu-reel shots, the browser report chart, the Godot matched shots, and the final comparison table. Feedback can change the reference workload, visual-equivalence allowance or whether Godot advances; it does not silently change the simulation authority or measurement contract.

## Handoff state

This branch is `t3code/explore-godot-renderer-spike`. The handoff includes the
browser graphics-test route, native GDExtension parity/IPC probes, the versioned
TypeScript capture contract, the Godot camera/workload probe, and the Godot
capture validator. Generated `.godot` caches, UID files, binaries and reports
were removed before handoff. The stopping point is after decoder checkpoint
`cb007edd`; no full-reel Godot renderer exists yet.

The next agent should start with `git status`, run the focused web tests and
`native/godot-spike/presentation_capture_test.gd`, then implement the capture
exporter at the existing publication seam. Preserve the explicit
`comparison_ready=false` guard until authored assets and capture playback are
actually rendered. Do not use headless Godot or the synthetic cube field for
performance conclusions.
