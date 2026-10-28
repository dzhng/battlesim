# Browser renderer shell

The browser entrypoint links the shared Rust renderer with presentation and
audio bindings in one WebAssembly module. The simulation worker remains a
separate authority. The [web manifest](../../web/package.json) owns compilation;
[`loadPresentation`](../../web/src/battle/sim/presentation.ts) owns initialization.

Device and surface admission can await browser work. World installation borrows
canonical presentation synchronously; a separate asset-preparation phase awaits
GPU impostor baking before drawing starts. Preparation and drawing borrow the
actual presentation during synchronous calls into the
[common Runtime](../renderer-core/README.md). A presentation borrow must never
survive asynchronous device or readback work.

The shell transports authored asset bytes and external geometry, configures the
canvas, acquires usable surface output and presents the runtime's display
texture. It acquires output before consuming incremental presentation work.
After device loss, admission creates a new device, installs current presentation
and prepares assets before drawing again. It does not restore a copied frame or
replay the last fog delta.

Explicit pixel, atlas and visibility queries queue GPU snapshots synchronously.
Their promises retain only the submitted readbacks and metadata, so the live
scene can render, resize or replace its catalog while mapping completes.

The [binding lifetime contract](../presentation-wasm/README.md) also applies to
renderer calls: allocation can grow the same memory used by UI and audio.
Rendering republishes the current frame header, and JavaScript reacquires typed
views before reading it. Browser diagnostics distinguish owned GPU payload,
retained external CPU payload and linear-memory high-water size.
