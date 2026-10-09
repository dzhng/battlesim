# Slice 02 — Spike the Rust-to-Godot binding architecture

## Contract unlocked

Select a native seam only after measuring it. Every candidate accepts commands, advances fixed ticks, returns side-filtered bulk observations and public geometry, and exposes replay/digest evidence. Simulation stepping, transfer/decode and teardown are timed independently.

## Candidates

Probe the smallest viable version of: (1) direct native Rust API or Godot GDExtension, (2) offline replay/presentation capture consumed by Godot, and (3) a separate Rust process with explicit IPC. Stop a candidate as soon as it cannot meet the contract; do not rank an easier demo against a complete client.

## Seam and ownership

Rust crates remain owners of rules, observations and digests. Godot owns window, input, rendering and host lifecycle only. Use bulk transfer and do not retain borrowed WASM-style views or copy packed layouts into a second schema without an explicit owner.

## Runnable artifact

A Mac command or Godot project starts, steps a fixed sample battle, exits cleanly and emits a machine-readable record containing command count, tick range, side, bytes transferred, decode time, sim time, teardown result, replay/digest result and candidate identity. The first offline artifact is `cargo run -p sim --release --example native_binding_probe -- <scenario.json> [seed] [ticks] [side]`; it proves the direct native owner can be measured without a renderer. The Godot 4.7.2 headless editor scan and live extension smoke pass from `native/godot-spike`; the smoke also proves invalid scenario input is refused. A paired browser/native record must still prove exact digest parity and side filtering before a live extension is selected. The renderer comparison additionally uses the versioned `battle-presentation-capture/v1` artifact, which carries workload identity, warm tick, side-filtered publication, digest and canonical camera pose downstream of Rust authority. Freeze the selected candidate's dependencies, versions and configuration.

## Verification

Write seam tests first: command ordering, fixed-tick count, side filtering, bulk transfer, backpressure/lifetime, clean shutdown, replay and exact digest. Run narrow Rust/native smoke checks plus the paired-record fixture. Recheck and cite official Godot renderer, GDExtension, godot-rust and benchmark documentation; pin versions and unsupported APIs.

If a candidate opens a window, use [compare-screenshots](../../../.agents/skills/compare-screenshots/SKILL.md), finish with [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md), and show evidence with [preview-shots](../../../.agents/skills/preview-shots/SKILL.md). Judge the image only for successful presentation and coordinate orientation; battle visual parity belongs to slice 03.

Choose the smallest candidate that passes authority parity, bulk transfer, lifecycle and measurement fidelity with no hidden gameplay authority. The spike selects GDExtension for the interactive client because it matches both side publications and replay/digest behavior in-process; the benchmark-only presentation capture remains the renderer measurement seam. IPC remains a fallback with clean process evidence, not a second interactive authority.

## Delegated choices

Exact ABI, serialization and project layout are delegated inside candidate probes. Candidate selection, parity checks, teardown and evidence schema are fixed.

## Must stay green

All Rust simulation/replay/digest tests, browser publication tests and slice-01 browser workload contract.
