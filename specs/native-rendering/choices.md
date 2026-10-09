# Native rendering choices

## User decisions — 2026-10-08

- Mac-first, platform-neutral spike.
- Godot versus the browser renderer first.
- Full main-menu reel as the reference workload.
- Graphics test is exposed in Settings for the current client only.
- Report average FPS, 1% low, absolute minimum, maximum and a frame-time chart.
- No hard performance threshold.
- Same gameplay and visual target; engine-specific rendering is allowed.
- Browser is the release baseline; Godot is experimental until broad parity.
- Binding architecture is itself the first spike; no random bridge is accepted.

## Delegated decisions

- Candidate binding seams are compared before selection: direct native/GDExtension, offline replay/presentation capture and process boundary. The winner is the smallest seam that preserves fixed-tick authority, side-filtered bulk observations, replay/digest parity and clean lifecycle.
- The live menu reel is not the timing driver. A canonical playback freezes its scene identities and camera data, ignores reduced-motion preferences and advances independently of render stalls.
- The existing synthetic benchmark is preserved as a separate workload because its identity and reports are already part of the repository's contract.

## Rejected or deferred

- Pixel-level cross-engine screenshots: too costly for the agreed visual target.
- A hard 2× or 25% performance gate: the verdict includes maintenance, startup, platform and visual evidence.
- Full browser/Godot live side-by-side comparison: each client reports its own current run.
- A general renderer backend abstraction before the spike proves a shared seam.
- Native JavaScript WebGPU and Rust `wgpu` in the first comparison: follow-up probes only if Godot evidence makes the host question material.

## Spike findings — 2026-10-09

- Godot 4.7.2 stable is pinned in ignored tooling for the spike and passes
  headless editor/GDExtension scanning. The workspace still has no native
  binding or Godot project. The first native pass therefore measures the
  existing direct Rust concepts and offline replay seam before a live window
  probe. This keeps the benchmark honest while the extension is built.
- Godot 4.7.2 is paired with godot-rust 0.5.5 `api-4-7`; the official
  compatibility guidance says the API version must be no newer than the runtime.
- The existing `game-wasm::BattleHandle` already exposes the required seam
  concepts: ordered commands, fixed stepping, side publication, digest,
  replay and observation resync. Native work should reuse those simulation
  owners rather than create a parallel authority.
