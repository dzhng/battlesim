# Browser simulation adapter

[The worker protocol](protocol.ts) carries ordered commands, acknowledgements and
completed-tick publications. [The authority](authority.ts) is shared by worker and
direct transports; direct transport is a testable host choice, not another rule
implementation. [The client](client.ts) owns the page's connection and subscriptions.

## Publication ownership

The authority reads the [WebAssembly boundary](../../../../crates/game-wasm/README.md)
into a newly derived memory view after publication, then copies into a credited
transfer buffer. WebAssembly memory can grow, so retaining its old typed view
across calls is invalid. Transfer hands buffer ownership to the consumer. The
consumer releases a publication only after its readers finish; until credits
return, the producer may stall rather than accumulate an unbounded queue.
The lab's session decodes every publication but renders only the latest once
an animation frame, so the page's cost per frame does not grow with the ticks
that arrived during it. React's copy therefore lags the presentation by up to a
render, and a drawn frame never reads it: everything a frame draws or decides
comes from the presentation's frame (`crates/presentation`) and the latest
publication its poses blend toward, so one frame shows one tick.

Rust decodes. [The presentation module](presentation.ts) pushes each publication
into the [publication reader](../../../../crates/sim/src/publication/reader.rs),
linked into the browser's rendering/audio WebAssembly module, and takes the
observation back as JSON in [the view's shape](observationView.ts). The fog bits and ground runs come
back as typed arrays, and unchanged fog and static groups keep their previous
views. Nothing on the page reads field offsets, enum rosters or packed bit grammars.

The side's [learned ground](../../../../crates/presentation/src/scars/) applies
side-local patches to one forward-moving view; it does not expose authoritative
ground changes a side has not learned.

Changing the observed side requires a fresh visibility/ground epoch. Old side
patches cannot continue into the new one. Command ownership remains separate from
a lab's observation-side switch. Completion acknowledgements and an actually drawn
frame are distinct: a probe must await the presentation it intends to measure.

## Lifetime

Presentation calls follow the [binding lifetime contract](../../../../crates/presentation-wasm/README.md).

[Module initialization](module.ts) is once per realm. [Preparation](../prepare/)
may supply an existing worker connection holding the admitted world; adopting it
avoids rebuilding the physical world to start a battle. Disposal releases the
WASM battle handle, credits and transport, and cancels outstanding ownership.

[Browser tests](../../../tests/) exercise commands, backpressure, side changes,
publication decoding and teardown. The real worker scene is a separate transport
proof. [The browser checking guide](../../../README.md#checks-and-evidence) owns
how to select either check and build its prerequisites.
