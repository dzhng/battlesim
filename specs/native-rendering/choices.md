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
- The first shared menu workload owner is a thin identity module over
  `fixtures/menu-backdrop.json`; it does not duplicate scene content or alter
  the live menu's clock.
- The first live binding seam is a Godot 4 GDExtension smoke crate using
  godot-rust 0.5.5 `api-4-7`, with an extension-owned `Node` and no simulation
  methods yet. It compiles for the target Mac; simulation transfer remains a
  later measured step rather than being hidden behind a fake API.
- The GDExtension smoke loads in Godot 4.7.2 headless and prints
  `rust-simulation-probe 0.1.0`; this validates the loader, entry symbol and
  Rust-owned node lifecycle. It does not yet prove simulation transfer.

## Audit entries — implementation passes

### Needs-user (low confidence)

- **The first live Godot binding returns a copied `PackedFloat32Array`.** The
  Godot API is convenient for a smoke method, so the pass chose to copy the
  simulation publisher's slice into Godot's packed-array value. The alternative
  would be a borrowed pointer or a shared native buffer. This is sound for the
  small probe but not yet a production transfer decision. Provisional call:
  keep the copy for the smoke and measure a bulk/shared-buffer alternative in
  the real binding slice; reverse it if transfer cost is material. The gap was
  that the spec required bulk transfer but did not select an ownership model.

### Sound (high confidence)

- **The offline native probe is a separate executable.** It advances the real
  Rust battle, copies one side publication, and replays the result to the same
  digest without opening a window. This keeps simulation authority evidence
  independent from Godot's renderer and makes a failed Godot installation
  diagnosable. The alternative would have mixed engine startup and sim timing.
- **Godot API 4.7 is pinned to the runtime.** The extension uses godot-rust
  0.5.5 with `api-4-7` because the verified runtime is Godot 4.7.2. The
  alternative of `api-custom` was rejected because the official compatibility
  guidance gives it no guarantee. This constrains the first Mac spike to the
  pinned runtime and leaves broader runtime support for a later decision.
- The existing `game-wasm::BattleHandle` already exposes the required seam
  concepts: ordered commands, fixed stepping, side publication, digest,
  replay and observation resync. Native work should reuse those simulation
  owners rather than create a parallel authority.

## Evidence decisions — 2026-10-09

- **Keep the interactive seam as a GDExtension, while using offline presentation capture for renderer comparison.** Same-input blue/red runs now match native and Godot digests and packed publication bytes, including replay, duplicate-command rejection, retained-buffer immutability and teardown. A minimal stdin/stdout process candidate also completes cleanly, but it adds process framing without improving the interactive contract. The offline capture is a measurement control, not a second gameplay authority; it must freeze the presentation history that raw observations alone do not contain.
- **Do not call the browser route a frozen reference yet.** The route now runs the full two-scene menu workload and uses the menu's subject smoothing, but its camera clock and plate composition still need to be aligned and its full production run and visual gates have not passed. Headless Godot and the 4096-cube scene remain smoke evidence only.
