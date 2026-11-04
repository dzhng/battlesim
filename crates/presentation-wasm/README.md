# Browser presentation bindings

These bindings link the [presentation crate](../presentation/README.md) into the
same WebAssembly module as rendering and sound. The battle authority's
[`game-wasm`](../game-wasm/README.md) remains separate in its worker; rendering
cannot inspect hidden simulation state.

The [browser entrypoint](../renderer-browser/src/) reexports the bindings, and
[`loadPresentation`](../../web/src/battle/sim/presentation.ts) owns its single
initialization promise per JavaScript realm. Presentation, renderer and audio
classes must come from that same generated module: a class from another binder
cannot safely carry a Rust object handle into it. The web manifest owns the
build; this crate is a linked library rather than another standalone artifact.

A side's publications become canonical Rust frames. The page reads frame
records through typed views over the exported memory, without an object per
entity. The renderer synchronously borrows those same Rust records; it neither
serializes them through JavaScript nor retains a presentation borrow across an
asynchronous device request. The browser wrapper keeps its Rust handle private
and exposes narrow rendering and audio operations.

Presentation, rendering and audio can all allocate in this memory. JavaScript
views must be reacquired after allocating calls because memory growth detaches
the old byte buffer. Renderer preparation can also materialize held static
batches, moving their record arrays. Both framing and rendering therefore
republish the current frame header before the page reads it again.

Presentation owns learned ground, fog, placement, poses and per-view selection.
The shared renderer consumes fresh incremental work after acquiring a drawable
output. The core owns [device reconstruction](../renderer-core/README.md#device-reconstruction).
The page still owns asset transport, input, UI observations and probes. Fielding
names the authored contents it needs, and the page fetches them without
reinterpreting their battle meaning.

External tooling art is admitted as an atomic catalog generation before its
models are requested. A decode failure leaves the active catalog usable.
Requests describe installed appearances, transforms and authored or explicit
poses; they supplement observed entities in the canonical frame. Texture
transport and GPU installation belong to rendering. Replacing tooling art
requires reinstalling renderer tables from the republished world header.
