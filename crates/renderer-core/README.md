# Shared renderer core

This crate owns the Rust `wgpu` renderer code shared by the native desktop and
[browser WASM shells](../renderer-browser/README.md). It has no `winit`, filesystem, audio, DOM or replay
dependencies. Platform shells provide surface and adapter admission,
runtime asset bytes and external geometry; the core synchronously borrows the
canonical presentation and owns resource composition, preparation, rebuilding
and frame encoding.

Authored model admission has one owner here. The platforms supply bundle bytes
and the presentation's appearance, pose and batch records; they do not reinterpret
static states, articulated nodes, corpse poses or material surface classes. The
native filesystem library is a source adapter to the same bundle decoder used
for browser fetches. The held static batch buffers also belong to this core, so
a frame that names an unchanged batch has the same meaning on either target.
Preparation detects skipped publications and materializes current canonical
populations before reusing held buffers; shells need no skipped-frame protocol.
Appearance rows route instances rather than own copies of their meshes. Rows
that select the same authored state/pose share immutable GPU tiers within one
asset generation; corpse poses remain distinct.
Decoded bundles also share immutable texture payloads by content identity.
The library's weak texture index cannot retain retired art. A new source draft
shares that index without copying canonical bundle ownership, so a late draft
cannot restore unrelated stale sources. Initial buffer data is queued through
the common upload owner, avoiding mapped-at-creation transfer copies in browsers.
Scenery also shares authored tier admission and held-batch routing. The runtime
reads the ground's kind order and stump meshes from presentation, preserving its
authority over placement.

[Impostors](src/impostor/) bake through the same authored model/material path,
then filter coverage before uploading relit cards. Native and browser use the
same atlases and shader; the impostor tier never substitutes a coarsest mesh.
Asynchronous asset preparation finishes before a frame may request cards.
Configurable offscreen bakes capture owned GPU resources, so replacing the
source catalog cannot invalidate a pending review image.

[Appearance replacement](src/runtime/assets.rs) stages a complete asset generation
against the presentation's prospective catalog. Asynchronous card baking owns
its resources and holds no live renderer or presentation borrow. Commit moves
matching canonical facts and GPU owners together, checks the actual device and
runtime/catalog identity, transfers current placement knowledge, reapplies
current controls, and requires a fresh
canonical frame before drawing the new rows. Targets, compiled pipelines, sky,
fog and learned ground retain their active owners throughout loading.

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
texture and requests for readback remain shell concerns; the core owns the
shared copy and mapping protocol. With `separate_high_air` enabled, high-air
models render to their own output for composition between UI planes.

Ground paint and display marks share geometry inputs but have separate depth
contracts. [Paint](src/paint.rs) sees world-only depth before units and grass;
painted materials sample its resolved color through the fog owner. X-ray counts
against that same world-only depth, then leaves silhouettes for
[display overlays](src/overlay.rs) to load and resolve even with no UI geometry.
Opaque display marks write a copy of physical depth taken before grass;
translucent marks only read it. Their information colors compose after post.

Pipeline construction uses the typed catalog and [composed WGSL](src/shaders/README.md)
owned by this crate. Native and browser call the same descriptor and
source owner; no shell reads a manifest or supplies shader text. The catalog
uses the actual `wgpu` binding, vertex, target, depth and multisample types.
The ABI test reflects WGSL structs named by the struct-bound entries in
`presentation::records::LAYOUTS` and the [renderer-owned records](src/records.rs),
checking member offsets, spans and fixed array counts recursively through nested
records wherever those structs appear in built-in shaders. A reverse check walks
every buffer binding, including nested record types; exceptions name their
format owner and reason in the ABI test. Directional shadow resources own their
depth array, receiver and cascade camera bindings. Cascade membership remains the
presentation's decision; the shared frame preserves it for ground, indexed
models and scenery.

On Windows, the [model pipeline registry](src/pipeline_registry/models.rs) selects
authored discard variants of cutout shaders with `cfg!(windows)`. AMD's Windows
driver crashes when creating depth-only sample-mask pipelines. Both variants
are composed and validated on every host; compiled target code selects the
variant without rewriting shader text.

The environment lookup data and adapted material algorithms retain their
[three.js MIT attribution](LICENSE.three).

The [source/artifact inventory](../../tools/renderer-accounting.mjs) separates
maintained shader source from expanded/generated payload. Actual route transfer
cost requires browser request evidence rather than adding every emitted chunk.

Fog sight also has one GPU owner. The presentation's canonical
[`FogUpload`](../presentation/src/fog/upload.rs) describes one drawn frame;
optional tables hold their previous values when absent. The runtime reads that
record directly from presentation in both native and browser WASM. Each drawn
frame needs a fresh upload so a completed
horizon rebuild does not run again.

## Device reconstruction

Device recreation rebuilds from current presentation. `Runtime::prepare`
materializes held static lists without reframing the battle, resends retained
scar state, and asks fog to resend its tables and rebuild its eye maps. The
latest delta alone cannot restore GPU-computed visibility.

## Rendering and diagnostics

[The water](src/water.rs) draws in its own pass after the world, whose passes
keep their samples for it. It copies the lit world as the world left it,
reduces that copy into mips and a guard (how unseen and how painted each pixel
under the water is), and refracts and blurs its bed only where the guard says
nothing unseen or marked would show through; `WaterDetail::Depth`, the lowest
tier, reads the copy at its own pixel and builds neither. Its wakes are a
height field stepped from `Presentation::waves`, taken once per drawn frame
because taking it advances the field's clock. A step takes a bounded number of
splats, so a big battle's wakes cost the same however frames batch the steps.

GPU profiling is opt-in. Whole-frame timestamp markers bracket the actual
ordered frame, including any regional scar submissions, and bounded asynchronous
readbacks skip saturated frames instead of waiting for the GPU. Resetting the
measurement window excludes pending samples from the old window.

[Explicit probes](src/probes.rs) inspect actual GPU sight, indirect arguments
and grass clumps. They queue bounded snapshot copies before mapping, so later
frames cannot change the result. Pending completion owns staging resources and
holds no renderer or presentation borrow; browser frames can continue while
readback waits for the event loop. Native polling runs only on explicit requests,
with a timeout. The shared [readback owner](src/readback.rs) also restores compact
RGBA rows from copyable display textures.

A fog probe can consume new canonical knowledge before any frame is drawn. The
GPU owner retains those uploads even for an inactive or empty query, and rebuilds
due horizon maps. Whole-structure flags and screen-tile lists still belong to the
next drawn view; probing never replaces the last view's diagnostic snapshot.

[Frame diagnostics](src/runtime/stats.rs) keep prepared population facts separate
from [actual issued world work](src/submitted.rs). Surface splits, scar regions,
independent depth and cascades repeat submissions without inventing new objects.
X-ray, ghosts and information marks stay outside that partial-world accounting.
Inspection uses the metadata captured during preparation, so a later publication
cannot silently relabel an already prepared scene. Platform shells add their own
resource payloads and measured GPU timings.

Diagnostic suppression preserves knowledge: hiding dynamic models leaves static
props and buildings, effects and their cast lights have independent switches,
and hiding material scars keeps consuming their uploads so restoring them shows
the current learned ground. Grass regrows when that material choice changes.

Memory diagnostics count buffers and textures held by the owners, deduplicating
shared handles and reading allocation descriptors. These are owned payload
bytes, not allocator or driver residency. The diagnostic visitor retains handles
only while taking a snapshot so diagnostics cannot prolong resource lifetimes.
The [browser binding lifetime contract](../presentation-wasm/README.md) owns
memory-view validity at the WebAssembly boundary.

```sh
cargo check -p renderer-core --target wasm32-unknown-unknown
```

Grass regrowth assigns draw ranges before emitting clumps. Tile order, then lane
and candidate order within each tile, determines both equal-depth winners and
which clumps fit a full tier. GPU scheduling must never choose either. The
renderer retains scratch ranges sized to the view's tile window; a held view
reuses its clumps without counting or emitting them again.
