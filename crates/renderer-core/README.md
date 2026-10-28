# Shared renderer core

This crate owns the Rust `wgpu` renderer code shared by the native desktop and
[browser WASM shells](../renderer-browser/README.md). It has no `winit`, filesystem, audio, DOM or replay
dependencies. Platform shells provide the surface, adapter/device policy,
runtime asset bytes and external geometry; the core synchronously borrows the
canonical presentation and owns resource composition, preparation, rebuilding
and frame encoding.

Authored model admission has one owner here. The platforms supply bundle bytes
and the presentation's appearance, pose and batch records; they do not reinterpret
static states, articulated nodes, corpse poses or material surface classes. The
native filesystem library is a source adapter to the same bundle decoder used
for browser fetches. The held static batch buffers also belong to this core, so
a frame that names an unchanged batch has the same meaning on either target.
Scenery also shares authored tier admission and held-batch routing. The runtime
reads the ground's kind order and stump meshes from presentation, preserving its
authority over placement.

Sky and environment lighting share one owner too. The sky lookup and its
reflection prefilter are baked from the presentation's light records at
construction. Each frame supplies the canonical view, camera and cast lights;
target shells neither reconstruct those uniforms nor approximate the sky's
rays or material light.

[`Runtime`](src/runtime/) constructs and prepares the complete renderer from
canonical presentation, source assets and external geometry. A held publication
reuses its model/effect buffers; a new canonical frame prepares them again.
Fog and scar work still prepare for every drawn frame because their incremental
work must be consumed rather than replayed.

[`FrameRenderer`](src/frame.rs) encodes a complete production frame over the
shared scene owners. Its concrete inputs carry GPU resources and canonical
presentation records; shells cannot substitute their own pass implementations.
The same owner routes model surface classes and fog, encodes cascades, grows
grass over the prepared scar pages, and submits regional ground draws when the
scar budget requires them. It completes post, the fog rim and optional player
overlays into one display texture. Surface admission, presentation of that
texture and readback remain shell concerns.

Pipeline construction accepts exported metadata plus a source provider, so the
native shell reads files while the browser shell supplies fetched strings.
Directional shadow resources own their depth array, receiver and cascade camera
bindings. Cascade membership remains the presentation's decision; the shared
frame preserves it for ground, indexed models and scenery.

Fog sight also has one GPU owner. The presentation's canonical
[`FogUpload`](../presentation/src/fog/upload.rs) describes one drawn frame;
optional tables hold their previous values when absent. The runtime reads that
record directly from presentation in both native and browser WASM. Each drawn
frame needs a fresh upload so a completed
horizon rebuild does not run again. Device recreation asks the presentation to
resend its tables and rebuild its eye maps; the latest delta alone cannot
restore GPU-computed visibility.

GPU profiling is opt-in. Whole-frame timestamp markers bracket the actual
ordered frame, including any regional scar submissions, and bounded asynchronous
readbacks skip saturated frames instead of waiting for the GPU. Resetting the
measurement window excludes pending samples from the old window.

Memory diagnostics count buffers and textures held by the owners, deduplicating
shared handles and reading allocation descriptors. These are owned payload
bytes, not browser or driver residency. The diagnostic visitor retains handles
only while taking a snapshot: a persistent JavaScript resource registry would
keep Rust's dropped WebGPU resources alive because that backend relies on garbage
collection. Browser CPU diagnostics report retained external geometry and decoded assets
separately; they do not claim allocator or whole-process memory. The browser
shares presentation, rendering and audio in one module, so any of them can grow
its memory. JavaScript must reacquire publication views after allocating calls.

```sh
cargo check -p renderer-core --target wasm32-unknown-unknown
```
