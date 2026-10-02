# Native rendering (placeholder)

**Status:** draft; no implementation started and no engine selected.
**Updated:** 2026-09-30.

## Next Agent Prompt

You are picking up a placeholder for evaluating native clients. When the user
starts this work, begin with the baseline spike below and turn the next spike
into a focused slice before implementing it. Read the repo and renderer skills
first. Recheck dependencies and platform support against their official sources.

The first outcome is evidence for a rendering approach, not a full port. Keep
experiments isolated from the browser product. Do not start a migration or build
a general renderer backend abstraction before the comparison identifies a need.
Record versions, runnable commands, results and decisions here, and update this
handoff before ending each pass.

- [ ] Freeze a representative browser baseline and its inputs.
- [ ] Spike native JavaScript WebGPU, including the window presentation seam.
- [ ] Spike a small Rust wgpu renderer using the same measured workload.
- [ ] Spike Godot native rendering and a minimal shared-sim binding.
- [ ] Compare matched workloads and propose the next implementation slices.

## Goal

Find out whether a native client offers enough performance, delivery or tooling
benefit to justify its implementation and maintenance cost. Evaluate how much of
the existing renderer can stay shared with the browser client.

The candidates are:

| Approach | What the spike must establish |
| --- | --- |
| TypeScript/TypeGPU with a native WebGPU host | Can the existing renderer run with limited host changes, and can it present directly to a native window? |
| Rust with wgpu | Can our WGSL, GPU layouts and pass design carry over, and how much orchestration must be rewritten? |
| Godot native with the Rust sim | What does an equivalent presentation workload cost, and what engine services justify a port? |

The existing browser client is the baseline and remains a supported client during
evaluation. A future full port, desktop packaging, complete HUD/audio integration
and Godot web support need a separate implementation plan after the verdict.

## Contracts and ownership

- **Simulation authority:** the existing Rust sim and shared contracts own the
  rules. Native clients send commands and consume side observations and public
  geometry. Engine physics, navigation or visibility must not become a second
  gameplay authority.
- **Publication boundary:** preserve commands, fixed ticks, observation semantics,
  replay and digest behavior. A native binding may bypass wasm-bindgen and call
  Rust directly; cross-target parity is something to verify, not assume.
- **Render input:** replay recorded presentation inputs without running the sim
  inside renderer timings. Start from the existing
  [scene seam](../../packages/battle-renderer/src/scene.ts); it contains GPU-specific
  objects and is not already a cross-engine transport format. Define only the
  smallest capture needed for each shared workload, with one source of camera,
  geometry, instances and visibility. Adapters consume that capture.
- **Host boundary:** device creation, window surface, frame scheduling, asset I/O,
  input and audio belong to the host. GPU passes remain in the existing renderer
  when testing JavaScript reuse. Inspect
  [device creation](../../packages/renderer-core/src/device.ts) for browser ties.
- **Assets:** reuse project-owned sources and validated appearance data. Any new
  third-party art follows the root `AGENTS.md` rule for it. Godot imports must preserve units,
  coordinate conventions and material intent.

Keep spike-specific adapters disposable. Promote shared seams only after a
candidate needs them; remove rejected harnesses and temporary bridges when their
evidence has been archived. There are no data migrations or compatibility shims
planned in this placeholder.

## Spike ladder

### 0. Freeze the baseline

Use the existing browser benchmark as the starting point. Freeze a commit,
dependency versions, assets, deterministic frame inputs, camera and render
settings. Identify a small workload that includes instanced geometry and one
compute pass, then a representative battle view with terrain, units, vegetation
and fog. Pin a target machine and an explicit workload budget before comparing.

**Artifact:** a reproducible baseline manifest, capture and timing report.
Separate simulation stepping, observation transfer, rendering CPU submission,
GPU execution and presentation. Describe which costs each timer actually covers.

### 1. Native JavaScript WebGPU

First reproduce the host's official compute/offscreen example. Dawn's Node
binding is a candidate for this probe, not a ready-made desktop window runtime.
Then run the small baseline workload through TypeGPU and identify API, shader,
asset-loading and capability differences. Record unchanged versus adapted code.

Next prove a native window surface and frame loop, including resize and cleanup.
Measure presentation separately. Routine pixel readback to display every frame
does not count as an acceptable native presentation path. If window integration
needs a custom binding or unsupported runtime, size that work explicitly.

**Artifact:** an offscreen capture plus a visible native window, with separate
verdicts for renderer compatibility and window-host feasibility. Only advance to
the representative existing battle renderer once both seams are understood.

### 2. Rust wgpu

Reproduce the upstream native example, then port the same small baseline workload.
Use the Rust wgpu API directly; wgpu-native is the related C-interface library.
Generate WGSL from the existing TypeGPU source where practical, rather than
maintaining two shader implementations. Verify buffer layout, bindings, coordinate
orientation, depth and compute output against the frozen baseline.

**Artifact:** a native window, matched capture and timings, plus an inventory of
the orchestration and host work needed for the full renderer. This spike does not
authorize translating the whole renderer into Rust.

### 3. Godot native

Run the relevant official native rendering benchmark, then build the same small
workload with an explicitly chosen rendering method and backend. Match geometry,
camera, resolution, lighting, shadows and visibility. Batch instances deliberately;
record unavoidable differences before treating timings as comparable.

As a separate probe, expose the existing Rust sim through a minimal GDExtension:
commands in, steps and side observations out, with replay/digest checks. Transfer
observations in bulk and measure transfer/decode independently from sim stepping.

**Artifact:** a native rendering capture and timing report, a runnable sim-binding
probe, and a list of custom passes/assets/UI that would need porting. A simpler
Godot image must not be ranked against our complete battle renderer.

### 4. Verdict and reslicing

Compare only workloads with equivalent outputs. Report visual gaps, code reuse,
implementation effort, maintenance ownership, platform reach and measured costs.
Keep offscreen, submission-only and displayed-frame results separate. Choose among
staying with the browser, adding a native WebGPU host, porting orchestration to
Rust, adding Godot, or gathering more evidence. No candidate is preselected.

**Artifact:** a short decision report and a fully sliced plan for the selected
next step. Browser/native dual support and the UI/audio host strategy must be
resolved before a full client port starts.

## Evidence and verification

- Freeze hardware, OS, drivers, graphics backend, build mode, versions, resolution,
  quality settings and input identities. Warm up pipelines, run candidates
  sequentially, repeat runs, and retain distributions rather than peak FPS.
- Measure CPU and GPU time separately; include frame pacing, memory, startup and
  shader compilation when comparing complete clients. Never equate a submission
  timer or readback benchmark with end-to-end frame rate.
- Before behavior changes, use the repo's write-tests workflow. Use replay/digest
  and observation checks for native bindings; investigate any cross-target drift.
- For visual acceptance, use compare-screenshots against the frozen baseline,
  specifying the workload's crop/mask and permitted differences. Use an unprimed
  screenshot-critique as the final visual check and preview-shots to show evidence.
- Keep temporary output in gitignored evidence directories. Archive the accepted
  baseline, manifest, captures and verdicts under this spec before selecting a
  direction, following the repo's Git LFS policy.

## Open decisions

The spikes should resolve the native JavaScript runtime and window binding,
TypeGPU compatibility, shader generation workflow, Godot renderer and extension
toolchain, and how much of the observation-to-presentation layer can be shared.
The user should choose the intended shipping platforms and acceptable maintenance
cost before the placeholder becomes a full implementation spec.

## Research starting points

Recheck these sources at spike time; external measurements are context, not our
performance targets.

- [Godot renderer architecture](https://docs.godotengine.org/en/stable/tutorials/rendering/renderers.html): Godot owns its rendering abstraction; it is not built on wgpu-native.
- [wgpu](https://github.com/gfx-rs/wgpu) and [wgpu-native](https://github.com/gfx-rs/wgpu-native): native WebGPU implementations and API boundaries.
- [TypeGPU](https://docs.swmansion.com/TypeGPU/): TypeScript orchestration and WGSL generation.
- [Dawn Node binding](https://github.com/dawn-gpu/node-webgpu): native WebGPU in JavaScript; browser canvas integration is not supplied.
- [Godot Rust bindings](https://godot-rust.github.io/book/): native shared-sim integration.
- [Godot benchmark suite](https://github.com/godotengine/godot-benchmarks): reproducible native workloads with separate CPU/GPU metrics.
- [Browser/native WebGPU experiment](https://github.com/SamG-Coder/three-runtime-benchmarks/blob/main/WEBGPU-REPORT.md): useful measurement distinctions; not a Godot comparison.
- [Apple M3 compute dispatch experiment](https://jelly48.com/blog/webgpu-dispatch-overhead): small dependent dispatches can remain costly natively.

Planning decisions and later spike findings live in [choices.md](choices.md).
