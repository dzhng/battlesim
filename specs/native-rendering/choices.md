# Native rendering choices

## Placeholder scope — 2026-09-30

The user requested a placeholder beginning with spikes. No engine or migration is
approved. The three candidates separate native execution from an engine change:
JavaScript/TypeGPU reuse, Rust wgpu orchestration, and Godot native presentation.

Keep the Rust simulation as the authority and preserve the browser client during
evaluation. Native JavaScript is the first renderer probe because it offers the
largest potential reuse; that ordering does not select it as the winner.

Window presentation gets its own feasibility verdict because an offscreen Node
WebGPU example does not establish a shippable desktop host. Matched rendering
workloads precede full-scene comparisons so omitted effects cannot masquerade as
performance gains.

This is a provisional ladder, not a completed implementation spec. Shipping
platforms and client maintenance budget remain open until the work is started.
Record later decisions with their evidence and rejected alternatives.
