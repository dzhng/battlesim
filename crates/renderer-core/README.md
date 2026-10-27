# Shared renderer core

This crate owns the Rust `wgpu` renderer code shared by the native desktop and
browser WASM shells. It has no `winit`, filesystem, audio, DOM or replay
dependencies. Platform shells provide the surface, adapter/device policy,
runtime asset bytes and presentation bridge; the core owns pass order, targets,
GPU resource lifetimes and draw encoding.

The extraction checkpoints own target formats, the sized target set,
buffer/bind helpers, pass debug-group orchestration, and pipeline construction.
Pipeline construction accepts exported metadata plus a source provider, so the
native shell can read files while the browser shell supplies fetched strings.
The reverse-Z depth prepass is also shared: `DepthPrepass` owns its pipeline
bindings and draw order while each shell adapts its ground, scenery and model
resources through `DepthInputs`.
The native client re-exports these owners while the remaining pass modules move
here incrementally. A browser-only copy of a native pass is not a valid
endpoint for the one-renderer spike.

Directional shadow resources are now one of those owners. `shadow::ShadowResources`
allocates the four-layer reverse-Z depth array, receiver uniform, per-cascade
camera buffers/bind groups, and comparison sampler. Native `SunShadow` adapts
presentation records and keeps its caster drawing policy in `native/client`;
the browser TypeGPU owner remains separate because it uses TypeGPU's bind-group
and command-encoder types. The resource contract is shared without inventing a
second browser pass graph.

```sh
cargo check -p renderer-core --target wasm32-unknown-unknown
```
