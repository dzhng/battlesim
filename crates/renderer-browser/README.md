# Browser renderer shell

The browser entrypoint links the shared Rust renderer with presentation and
audio bindings in one WebAssembly module. The simulation worker remains a
separate authority. The [web manifest](../../web/package.json) owns compilation;
[`loadPresentation`](../../web/src/battle/sim/presentation.ts) owns initialization.

Device and surface admission can await browser work. World installation,
preparation and drawing instead borrow the actual canonical presentation during
synchronous calls into the [common Runtime](../renderer-core/README.md). A
presentation borrow must never survive an asynchronous device request.

The shell transports authored asset bytes and external geometry, configures the
canvas, acquires usable surface output and presents the runtime's display
texture. It acquires output before consuming incremental presentation work.
After device loss, admission creates a new device first; the next synchronous
draw reconstructs runtime resources from current presentation and retained
external sources. It does not restore a copied frame or replay the last fog
delta.

The [binding lifetime contract](../presentation-wasm/README.md) also applies to
renderer calls: allocation can grow the same memory used by UI and audio.
Rendering republishes the current frame header, and JavaScript reacquires typed
views before reading it. Browser diagnostics distinguish owned GPU payload,
retained external CPU payload and linear-memory high-water size.
