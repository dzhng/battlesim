# WebAssembly boundary

This crate exposes shared physical preparation and the battle authority to
JavaScript. [The binding source](src/lib.rs) owns exported calls; rule and
admission implementations stay in [simulation](../sim/README.md),
[contract](../contract/README.md) and [mapgen](../mapgen/README.md).

Preparation admits maps and skirmish placement through those same native owners. A plain
world supplies geometry exports and physical queries for page drawing and picking;
it is not a second running battle. [Browser preparation](../../web/README.md#authority-and-preparation)
keeps expensive construction in its owned worker and adopts that prepared world
into the battle authority.

Rare control messages cross as JSON. Tick publications use a reusable packed
buffer and the layout declared by their Rust producer. Read a publication before
another WebAssembly call can grow memory or replace the buffer; never retain a
borrowed memory view as a durable frame. [The browser adapter](../../web/src/battle/sim/README.md)
owns copying, credit transfer, decoding and handle lifetime.

The build task in [the web manifest](../../web/package.json) owns wasm-pack and
its target/output settings. Generated bindings under `web/src/wasm/` are rebuildable
output, not edited source. Each page or worker initializes its own realm's module;
sharing a JavaScript loader promise does not share an authority between workers.

A Rust panic traps the call that hit it, and the handle it held stays borrowed:
every later call then fails with "recursive use of an object", which names
nothing. [The binding source](src/lib.rs) installs a panic hook that rethrows the
panic's own message, uncaught, so the page hosting the worker records it with
its other errors (and its diagnostics report).

[Paired native/WebAssembly records](../../fixtures/README.md#paired-records) prove
both execution and packed decoding. Float32 observation agreement alone cannot
prove identical authoritative state; exact battle digests provide that check.
