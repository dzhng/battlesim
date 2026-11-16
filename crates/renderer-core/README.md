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

Appearance rows route instances rather than own copies of their meshes. Each
runtime owns one [art store](src/models/store.rs) for its scene and device,
shared with pending generations. Four rules hold for everything in it: packed
geometry, material slots, and [texture and card layers](src/texture_array.rs).

- **Identity is content, never a name or a row.** What is packed, uploaded or
  baked for a content depends only on the inputs to those bytes. Geometry
  retains the authored skin; living and corpse rows share it, while cards
  include their rendered rest palette. A type added to a running battle costs
  its own art alone. A key includes
  every input that changes the stored bytes; a texture array's edge is one,
  because it changes filtered pixels.
- **Addresses are append-only.** A material slot or a layer never moves and is
  never reassigned while the store lives, and a dropped preparation keeps what
  it reserved, so cancelled work cannot renumber live art. A layer is written
  before any committed table can name it and is immutable afterwards.
- **The last holder destroys.** Generations hold the GPU payloads and the
  store's indexes are weak. Retiring a generation cannot free what another
  draws.
- **A full rebuild is the same admission with a fresh store.** It runs for the
  first scene, a recreated device, and when dead or exhausted addresses pass
  the store's policy; admission work names the reason. An address that cannot
  fit the vertex layout is an error, never a wrap.

Materials, row routing and diagnostics stay per generation, rebuilt against the
stable addresses. Growing an array copies its layers on the GPU and never
uploads old art again; a changed edge rebuilds the array. Admission work counts
both apart from new-layer writes.

Decoded bundles are keyed by the runtime catalog's `ContentId`: the bundle hash
and, for skinned art, its skeleton hash. Names only route to that identity, so
one name can replace its art and two names can share one decode. The shell asks
which identities are missing from both the decoded library and presentation's
retained facts, and supplies bytes for those alone;
[source drafts](src/bundle/draft.rs) give browser transport and the native
tests that one answer. A draft holds no retired art and refuses retired content
before any GPU work.

Initial buffer data is queued through the common upload owner, avoiding
mapped-at-creation transfer copies in browsers.
Scenery also shares authored tier admission and held-batch routing. Initial
placement and atomic art replacement derive geometry from decoded Rust bundles;
presentation owns the resulting sizes, trunks, stumps and placement. Runtime
reconstruction retains those facts and the observer's learned felling.

[Impostors](src/impostor/) bake through the same authored model/material path,
then filter coverage before uploading relit cards. Native and browser use the
same atlases and shader; the impostor tier never substitutes a coarsest mesh.
Runtime cards live in the art store under the same rules. A card's key adds the
bake specification and the rendered rest palette to its geometry's. The
name-bearing atlas hash is a review oracle, not a cache key.

A stage bakes only the keys the store lacks, in batches bounded by the memory
their targets and readbacks hold, and awaits each batch's reads together; the
bound's default lives with the [impostor specification](src/impostor/mod.rs),
and a split is reported once per runtime. Authored materials upload once per
generation and stay separate from live diagnostic material controls. A review
is always an independent bake, never a read of a stored layer.
Asynchronous asset preparation finishes before a frame may request cards.
Configurable offscreen bakes capture owned GPU resources, so replacing the
source catalog cannot invalidate a pending review image.

[Appearance replacement](src/runtime/assets.rs) stages a complete asset generation
against the presentation's prospective catalog. Pose data follows
[presentation's sharing contract](../presentation/README.md#what-it-owns), so
staging a renderer generation does not duplicate the scene's clip tables.
A pending generation can warm
one missing geometry key and its rest facts per step, retaining those rows
until the existing admission finds every packed key. Dropping partial work
releases its holders without changing live art; stepping changes neither the
complete admission report nor committed pixels. Asynchronous card baking owns
its resources and holds no live renderer or presentation borrow. Commit moves
matching canonical facts and GPU owners together, checks the actual device and
runtime/catalog identity, transfers current placement knowledge, reapplies
current controls, and requires a fresh
canonical frame before drawing the new rows. Targets, compiled pipelines, sky,
fog and learned ground retain their active owners throughout loading.

Static admission compares the presentation's ground revision, each kind's
content identity in order, and grass density. Unchanged static content does no
placement and no scenery or grass upload: scenery keeps its GPU buffers while
its rows follow the prospective catalog, and grass keeps its live owner, so a
frame prepared during admission cannot lose its latest regrowth.

Sky and environment lighting share one owner too. The sky lookup and its
reflection prefilter are baked from the presentation's light records at
construction. Each frame supplies the canonical view, camera and cast lights;
target shells neither reconstruct those uniforms nor approximate the sky's
rays or material light.

[`Runtime`](src/runtime/) constructs and prepares the complete renderer from
canonical presentation, source assets and external geometry. A held publication
reuses its model/effect buffers; a new canonical frame prepares them again.
Fog and scar work prepare for every drawn frame because their incremental
work must be consumed rather than replayed.

Each asset generation also carries an [`AdmissionWork`](src/runtime/work.rs)
report: counts and payload bytes for decode, packing, uploads, texture layers,
cards, card-read wait groups and scenery placement. It is swapped atomically
with the generation, so discarded or stale preparation cannot change the live
report.

[World geometry](src/runtime/world.rs) constructs submitted layers for both
shells through shared skirt, backdrop and water builders. Native supplies physical
triangles; the browser supplies the same Rust-generated terrain mesh it uses for
picking and shadow fitting, plus explicit boundary positions and extra faces.
Water cells come from canonical ground while the browser borrows presentation
synchronously; empty cells omit water.

[`FrameRenderer`](src/frame.rs) encodes a complete production frame over the
shared scene owners. Its concrete inputs carry GPU resources and canonical
presentation records; shells cannot substitute their own pass implementations.
The same owner routes model surface classes and fog, encodes cascades, grows
grass over the prepared scar pages, and submits regional ground draws when the
scar budget requires them. It completes post, the fog rim and optional player
overlays into one display texture. Shells own surface admission, presentation
of that texture and requests for readback; the core owns the shared copy and
mapping protocol. With `separate_high_air` enabled, high-air
models render to their own output for composition between UI planes. The
world's effects draw again over that output, soft against the models' depth
and only where they cover it, so what sits in front of a jet (its own nozzle
glow and flame, smoke) is not hidden by the plane.

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

### GPU timing

GPU profiling is opt-in and owned by [`FrameTimer`](src/timing.rs). Whole-frame
render-pass timestamp markers bracket the actual ordered frame, including any
regional scar submissions. Stage counters measure elapsed time between marker
passes while GPU stages can overlap; GPU traces provide the busy/idle breakdown.

The browser uses `Resolve::InFrame`: resolve in the marked encoder and publish
rolling statistics asynchronously, without waiting. Chromium/Metal comparisons
found no missing duration with this path. Native uses `Resolve::AfterCompletion`
because its Metal backend can return stale counters when resolving in-frame.
Its explicit ticket wait drains the render, resolves in a separate submission,
then waits for readback. These profiling waits affect throughput; compare timed
CPU phases and GPU duration separately from instrumented frame intervals.

Callers size the bounded readback pool for their pacing; saturation skips a
measurement. Each native ticket owns its duration independently of completion
order. Resetting rolling statistics excludes every frame begun before the reset
without cancelling its pending result. Invalid counters or failed maps remain
missing measurements rather than zero-duration frames.

### Inspection and diagnostics

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

Grass regrowth assigns draw ranges before emitting clumps. Tile order, then lane
and candidate order within each tile, determines both equal-depth winners and
which clumps fit a full tier. GPU scheduling must never choose either. The
renderer retains scratch ranges sized to the view's tile window; a held view
reuses its clumps without counting or emitting them again.

## Checks

The [integration tests](tests/) own shared renderer behaviour, including GPU
pipeline construction, rendered pixels, readback and timing. They use headless
admission with the production device policy. Native backend features and the
blocking executor are test dependencies; production WASM keeps its browser-only
backend selection. Tests that need desktop replay, filesystem adapters or audio
belong to the [native shell](../../native/client/README.md#checks).

Asset-backed tests read the prepared runtime, so materialize its Git LFS files
before running the suite. The tests need no GPU lock; [only a frame-timing
measurement takes it](../../web/README.md):

```sh
git lfs checkout assets/runtime
CARGO_INCREMENTAL=0 cargo test -p renderer-core
cargo check -p renderer-core --target wasm32-unknown-unknown
```
