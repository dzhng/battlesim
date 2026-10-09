# Native rendering and Godot evaluation

**Status:** slice 01 in progress; browser frame-rate contract landed, menu workload and native probes remain.
**Updated:** 2026-10-09.

## Next Agent Prompt

The first pass added the shared displayed-frame rate fields and tests. Finish slice 01 by freezing the menu-reel manifest and wiring Settings to the full reel. Slice 02 can now use the verified Godot 4.7.2 headless binary in `throwaway/tools/godot-4.7.2` plus the offline native probe; keep the existing synthetic `/benchmark` workload unchanged. Update this handoff, the choices ledger and the owning slice before ending each pass.

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
- [~] Freeze menu-reel manifest, capture schema and browser control — slice 01 (manifest landed).
- [ ] Add Settings graphics test and report chart — slice 01.
- [~] Verify offline native seam and Godot headless toolchain — slice 02 (offline probe and live extension smoke load; real scenario transfer/parity remains).
- [ ] Spike binding candidates and select a seam — slice 02.
- [ ] Render the full menu reel in Godot — slice 03.
- [ ] Compare repeated distributions and reslice the chosen direction — slice 04.

## Human review surface

The human reviews the frozen menu-reel shots, the browser report chart, the Godot matched shots, and the final comparison table. Feedback can change the reference workload, visual-equivalence allowance or whether Godot advances; it does not silently change the simulation authority or measurement contract.
