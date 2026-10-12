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
as JSON on demand. The side's learned ground stays in the module: the terrain asks
`scars_changed`, `scar_regions` and `prepare_scars` each frame and uploads the
`ScarUpload` it gets, and the page reads the cleared runs and marked cells.

It links the [publication reader](../sim/src/publication/reader.rs) and the
presentation crate, and no rules, so the main thread never loads the simulation.
The web manifest's `build:wasm` builds it after `game-wasm`, into
`web/src/wasm/presentation/`.

It also hands the page the static world `static_world` resolves: `StaticWorld`
is the buildings and their walk; `Ground` is a map's ground, built once per map
from `WorldView`'s exports, the world layout, the biome file and the map's
region: the terrain's GPU tables, the grass's (from the renderer's facts about
its blade meshes), the water's cells, and the scenery, the dressing laid a cell
at a time on request. Its probes (`plots_json`, `distances` and the like) serve
the ground scenes; no frame reads them.
