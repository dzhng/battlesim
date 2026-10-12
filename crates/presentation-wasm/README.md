# Presentation module

The page's own WebAssembly module, beside the battle's
([`game-wasm`](../game-wasm/README.md)), and the browser binding of the
[presentation API](../../specs/native-rendering-parity/slices/18b-presentation-api.md)
over [the presentation crate](../presentation/README.md). Each tick's publication
goes in through `push_publication`; `clock` gives the presentation time and
`frame` returns the address of a `FrameHeader` whose sets the page reads as typed
views over this module's memory (`records.gen.ts`'s `frameViews`), with no object
per entity. A view lasts until the next call into the module, which may grow its
memory. The game UI's observation comes out of `observation()`, as JSON in the
shape of `web/src/battle/sim/observationView.ts`, with the fog bits and ground
runs as typed arrays beside it; sound's publication and the labs' probes come out
as JSON on demand.

It links the [publication reader](../sim/src/publication/reader.rs) and the
presentation crate, and no rules, so the main thread never loads the simulation.
The web manifest's `build:wasm` builds it after `game-wasm`, into
`web/src/wasm/presentation/`.

It also hands the page the static world [`static-world`](../static-world/) resolves:
`Ground` is a map's ground shapes (forest shapes, the plot split, the paved strokes
as drawn, surface fields and water cells) as flat arrays, built once per map from
`WorldView`'s exports and the regional biome.
