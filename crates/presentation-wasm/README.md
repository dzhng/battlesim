# Presentation module

The page's own WebAssembly module, beside the battle's
([`game-wasm`](../game-wasm/README.md)), and the browser binding of the
[presentation API](../../specs/done/native-rendering-parity/README.md#the-presentation-api-18b)
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

What a battle's units and effects draw from is `Fielding`'s
([`presentation::fielded`](../presentation/src/fielded.rs)): it names the
runtime contents it reads (`contents`), the page fetches and inflates them as
it does any bundle, and `resolve` returns `load_world`'s `units` and `effects`
and the clip tables. The page decides nothing about them.

It links the [publication reader](../sim/src/publication/reader.rs) and the
presentation crate, and no rules, so the main thread never loads the simulation.
The web manifest's `build:wasm` builds it after `game-wasm`, into
`web/src/wasm/presentation/`.

The same `Presentation` holds the scene's static world: its buildings
(`add_buildings`), and its ground (`set_ground`, from `WorldView`'s exports, the
world layout, the biome file and the map's region), whose terrain and grass GPU
tables (`ground_terrain`; `grass_params` over the grass kinds' shapes, which
`GrassShapes` packs from their bundles' tiers) and water mesh the page reads
once, each frame's wave packet (`waves`: the wave field's steps and the
wakes' splats), and each frame's grass window (`grass_window`). Its scenery
(`place_scenery`, given each kind's finest mesh, off which Rust reads its size,
a tree's trunk and crown, and its stump, `stump_mesh`) the frames' static
batches carry, with the buildings'; the scenery follows the ground the side
has learned cleared by itself. The player's scenery settings
(`set_scenery_quality`) are its input. Its `ground_` probes (`ground_distances`
and the like) serve the ground scenes; no frame reads them. The ground's other
meshes (terrain, skirt, backdrop) and the map's box are free functions, as are
the tier, caster tier and fog group a lab's own models take (`model_tiers`,
for the frame's `View` record; `batch_caster`; `model_fog`), which have no
frame to carry them. Given
the light and the ground's mesh (`set_shadow`), every frame carries the camera's
view and uniform and the sun's cascades fitted to it.
