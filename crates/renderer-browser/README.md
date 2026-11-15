# Browser renderer shell

The browser entrypoint exposes the shared Rust renderer with presentation and
audio bindings. The simulation worker remains a separate authority. The [web manifest](../../web/package.json) owns compilation;
[`loadPresentation`](../../web/src/battle/sim/presentation.ts) owns initialization.

The page retains one `BrowserGpu`, admitted asynchronously without a canvas.
Each `Renderer` receives that owner and its canvas(es); detached workbench canvases
share the same device. Dropping a renderer releases its scene, while page departure
explicitly disposes the device. Browser WebGPU frees an allocation only on
`destroy()` or garbage collection, so the core's owners of large allocations
destroy them when they drop: a rebuilt scene or a resize returns its memory at
once, however often a page rebuilds. JavaScript receives owned diagnostics, pixels and
query results rather than GPU handles. The owner's failure promise reports the
first unexpected GPU error or loss; deliberate disposal resolves it without an
error. Production treats unexpected failure as terminal until manual Reload.

Device and surface admission can await browser work. World installation borrows
canonical presentation synchronously; a separate asset-preparation phase awaits
GPU impostor baking before drawing starts. Preparation and drawing borrow the
actual presentation during synchronous calls into the
[common Runtime](../renderer-core/README.md). A presentation borrow must never
survive asynchronous device or readback work.

The browser shell exposes the committed generation's admission work to the page;
the page adds transport and appearance-library work to its diagnostics event.

The shell transports authored asset bytes and geometry inputs, configures the
canvas, acquires usable surface output and presents the runtime's display
texture. It acquires output before consuming incremental presentation work.
The recovery harness explicitly admits a fresh `BrowserGpu` after device loss,
then installs current presentation and prepares assets before drawing again.
A renderer cannot silently replace the page's shared device.
The new runtime starts with a fresh art store and reports device recreation as
its rebuild reason. Failed installation keeps that reason for the next attempt.
The core owns [device reconstruction](../renderer-core/README.md#device-reconstruction).

[Appearance drafts](src/appearance.rs) carry declarations for every wanted name
and its runtime `ContentId`; `Renderer::missing_content_ids` answers which
identities lack either decoded source or committed presentation facts. Both
`missing_content_ids(presentation, json)` and `appearance_draft(presentation,
detail)` consult the same shared source admission owner. Initial renderer
installation requires facts for every loaded content identity. A draft resolves
held content against the current committed owners when prepared; retired
content refuses preparation without changing live art. `AppearanceDraft::add_bundle` supplies facts and
bytes by identity, so a draft may send only that missing list while the page can
still send all bytes. Names route to the bundle hash (and a skinned bundle's
skeleton hash), never to a reuse decision. Drafts collect source transport and
pose facts without changing active art. Page-generated bundles use the same
exported `bytes_hash` implementation as the contract crate, cached by bundle
object in the web renderer. Preparation returns an owned promise; the caller
may continue rendering or dispose its scene while cards bake. Latest-request
scheduling belongs to the viewport. It commits only its desired complete
candidate and latest model requests at a frame boundary, then publishes a fresh
canonical frame. Retained source ownership changes with the GPU/catalog commit,
so recovery cannot resurrect an older generation.

Explicit pixel, atlas and visibility queries queue GPU snapshots synchronously.
Runtime-spec atlas reviews inspect resident card layers when present; other
specifications use the independent authored bake. The result identifies a
resident layer separately from its name-bearing pixel hash.
Their promises retain only the submitted readbacks and metadata, so the live
scene can render, resize or replace its catalog while mapping completes.

Renderer calls follow the [binding lifetime contract](../presentation-wasm/README.md).
Browser [memory diagnostics](src/diagnostics.rs) report owned GPU payload and
retained CPU geometry and decoded-asset payloads, excluding presentation,
allocator overhead and driver residency.
