# Shared renderer core

This crate owns the Rust `wgpu` renderer code shared by the native desktop and
browser WASM shells. It has no `winit`, filesystem, audio, DOM or replay
dependencies. Platform shells provide the surface, adapter/device policy,
runtime asset bytes and presentation bridge; the core owns pass order, targets,
GPU resource lifetimes and draw encoding.

Authored model admission has one owner here. The platforms supply bundle bytes
and the presentation's appearance, pose and batch records; they do not reinterpret
static states, articulated nodes, corpse poses or material surface classes. The
native filesystem library is a source adapter to the same bundle decoder used
for browser fetches. The held static batch buffers also belong to this core, so
a frame that names an unchanged batch has the same meaning on either target.
Scenery also shares authored tier admission and held-batch routing; shells
supply the ground's kind order and presentation-owned stump meshes. Keeping
those inputs separate preserves presentation's authority over placement.

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
