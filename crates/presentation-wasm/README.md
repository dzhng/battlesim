# Presentation module

The page's own WebAssembly module, beside the battle's
([`game-wasm`](../game-wasm/README.md)), and the browser binding of the
[presentation API](../../specs/native-rendering-parity/slices/18b-presentation-api.md).
Each tick's publication goes in through `push_publication`. The game UI's
observation comes out of `observation()`, as JSON in the shape of
`web/src/battle/sim/observationView.ts`. The fog bits and ground runs come out
as typed arrays beside it.

It links the [publication reader](../sim/src/publication/reader.rs) and no rules,
so the main thread never loads the simulation. Dead-code elimination keeps it
near 260 KB. The web manifest's `build:wasm` builds it after `game-wasm`, into
`web/src/wasm/presentation/`.
