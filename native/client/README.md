# Native client

The native client on `wgpu`. It runs the browser's own shaders and passes over the [presentation crate](../../crates/presentation/README.md)'s
`World` and `Frame`, and decides nothing per entity itself.

## Windows development

The native client is a Windows desktop binary built with the MSVC Rust target
and Visual Studio's C++ workload (the Windows SDK supplies `link.exe`). From a
Windows PowerShell checkout, run `powershell -ExecutionPolicy Bypass -File
tools/setup-windows.ps1 -Native`; the script also prepares the browser path.
The GNU target can be built from WSL after installing MinGW, but MSVC is the
supported Windows development target.

The browser and native clients share the same baked runtime files but load them
through different boundaries. Vite copies `assets/runtime` into the web build;
the native client reads it from disk. A packaged native build should place
`assets/runtime` above the executable or set `BATTLEGAME_RUNTIME` to its path.
The native resolver no longer depends on the WSL checkout path.

The browser is started with `bun run dev`. WSL should serve the app while
Windows Chrome supplies the real GPU; configure `.wslconfig` with
`networkingMode=mirrored` so Chrome can reach the WSL localhost server. WSL's
own software adapter is suitable for pipeline checks, not for the full battle.

On Windows, the native client adapts the browser's cutout sample-mask shaders
to discard-based fragments while compiling pipelines. AMD's Windows driver
crashes in `wgpu` when those depth-only sample-mask pipelines are created; the
adaptation preserves the cutout test and leaves the browser export unchanged.

- **Shaders and pipelines** are the browser's [export](../../crates/pipelines/README.md),
  built from `pipelines.json` at start, with the sizes the passes allocate by
  (the bloom's levels, the sky lookup, the effect sheets). The export is read
  strictly: a setting the client does not know fails the build rather than
  being dropped, and any pipeline the device refuses fails the start, listing
  each one.
- **The frame** is one ordered list of passes, encoded in order each drawn
  frame, as Bevy draws and as the browser's `battleFrame` does. It
  has no render graph. A pass reads the frame's `View` and the sized targets.
- **Uniforms** are records the presentation crate resolves (the sky's light,
  the sun, the haze, the bloom, the grade, the view's sky rays). The client
  uploads their bytes as they are, and `crates/pipelines` holds each record to
  the struct its shader declares. A pass binds groups by the layout they fill,
  never by number.
- **Models** are the browser's appearance bundles from `assets/runtime` (Git
  LFS; pull the ones a scene needs), read through the presentation crate's
  runtime reader (`presentation::assets`) as the browser reads them: the
  catalog, gzip transport, the bundle container and its texture files. Every
  `World.appearances` row is loaded once into the browser's vertex layout, and
  each `Frame` batch is one run of instances over one row's tier. A corpse is
  its body posed once by the presentation crate's rest palette; so is a body
  at the impostor tier (its far pose), since impostor cards are not baked yet.
- **The ground** is the presentation's (`Presentation::set_ground`), from
  the saved map as the simulation's own world code exports it
  (`static_world::files::ground`, what the page reads from `WorldView`): the
  terrain's tables, its meshes (terrain, skirt, backdrop, water), the grass's
  tables and blade shapes, and the side's learned scars. The client uploads
  them and draws the terrain into the depth prepass, the cascades and the lit
  world. The grass grows on the GPU over the window the crate fits to each
  view, through the exported build, finish and indirect draw.
- **The water** is the browser's shader over the presentation's water mesh
  and looks, drawn `full` (`WaterDetail`): after the world, refracting a
  mipped copy of it and the fog before it. Native draws no wakes; what is
  left to port is the [water native](../../specs/water-native/README.md)
  list.
- **The scenery** (trees, the tree lines' shrubs, the scenery past the map,
  the forest floor's dressing, felled trees and their stumps) is the frame's
  static batches. The replay places it from each kind's finest mesh
  (`place_scenery`), packed as the browser packs it; the presentation walks
  it and cuts each felled tree's stump (`stump_mesh`). Each row's batches
  draw through the browser's scenery pipelines, routed by
  `Appearance.draws`, into the depth prepass, the lit world and, for what the
  walk lists as casters, the cascades. The presentation clears shrubs and
  dressing off the ground the side has learned cleared by itself.
- **The camera and sun shadows** come with each frame: its `View`, its
  camera uniform, and four cascades fitted over local terrain receivers
  (`presentation::shadow`, given the light and shared terrain partition, `set_shadow`),
  the receiver block and each cascade's caster camera, all uploaded as they
  are. Each batch names the tier it casts with and the fog group it binds
  (`Batch.caster`, `Batch.fog`); the client routes by them.
- **Buildings** are the frame's static batches too: the map's buildings
  placed from the template art, in the state the side knows.
- **Props** (parked cars, lamps, walls, fences, hedges, benches, wrecks,
  rubble, shoved bodies) are the frame's static batches as well: the replay
  places the map's props (`place_props`) from the art a page running the
  scene's units installs (`files::prop_art`: the catalog's footprints,
  paints and wrecks, each look's bounds read from its bundle's header), and
  the presentation fits them and adds what the side knows. Their rows
  (`Appearance.draws` 7 and 8) draw through the model pipelines. Independent caster batches preserve
  offscreen shadows and exclude props beyond receiver reach; camera batches
  never cast a second copy.
- **The static batches are held between frames** (`statics.rs`), as the
  browser holds them: one buffer per batch key, rewritten only when the
  frame writes it, dropped when the frame stops listing it. The models and
  the scenery draw them from there. A replay that jumps between plates
  frames the records between without drawing them, so each undrawn record
  bumps the frame's static generation and the next frame writes every
  static batch anew.
- **Effects** are the frame's effect instances (`Presentation::effects`),
  drawn into the resolved lit world over the browser's own flipbook sheets
  (`assets/third-party/effects`, Git LFS), and the cast lights the
  presentation chose (`Presentation::lights`) light every lit material and
  the smoke.
- **Fog of war's sight** is the presentation's (`Presentation::prepare_fog`):
  each drawn frame the client uploads what it is handed (eyes, occluders,
  rebuild records, the structures' grid and `FogParams`) and runs the
  browser's `fogTerrain`, `fogMerge`, `fogWhole` and `fogCull` after the
  depth prepass. Every fogged material binds the fragment group of what it is
  (`fog::Layer`): units never fogged, corpses as the ground under them, the
  static world face by face. A replay draws fog where the browser did (the
  manifest's `fog` layer), the side's foliage following the ground it sees
  cleared.
- **Fog's mask** runs as the browser's does: the world resolves its fog mask
  beside the lit world, and the mask passes compose the fogged world into
  post's input and draw the rim after post, in the fixture's unseen look
  (`presentation::fog` resolves it).
- **Sound** is the presentation crate's: it decides every voice, and the
  client's `audio` module carries the commands out on `kira` (a track per
  voice under its bus, low-pass, equal-power pan, reverb sends, a limiter),
  with the recordings in `assets/runtime/audio/clips` (Git LFS) and the
  synthesized sounds composed by the same Rust bank the browser's decisions
  were made against. Voice lines (the crate's answers and barks) are takes
  of the voice packs (`assets/runtime/audio/voices/<set>/pack.json`), read
  and decoded when a line plays; the score (the crate's mood and track
  choice) streams its MP3s from `assets/runtime/audio/music` on a music bus
  that crossfades and dips under lines. A track or take Git LFS has not
  fetched is refused by name in the report. `--sound <wav>` renders a
  replay's sound to a file; it is never played aloud by the replay, and the
  tests render to a buffer.
- **What is bound but not drawn yet** reads as absent in every shader: the
  painted ground marks (the UI paints them; a replay has no UI).

## Running a parity scene

The binary replays one scene of a browser parity run (the control the
[spec](../../specs/done/native-rendering-parity/README.md) names). It refuses a
capture or a saved map that is not the one the manifest's identity hashes,
naming the file, and reads the capture and the manifest as strictly as the
browser does: a key the format lacks or leaves out refuses the file. It reads
the world from the repository, not from the run: the map, its ground, buildings
and props, and the units its encounter fields with their effects
(`presentation::fielded`). Then it frames every record through the presentation
crate and writes each plate the browser wrote, under the same name. A plate
whose counts differ from the browser's fails the run: it exits with an error
listing each difference after the report. Its usage text lists the flags. Run it
under the GPU lock.

The window opens borderless and off-screen by default, so nothing appears on
the user's screen. On macOS, `wgpu` presents nothing to a window that isn't
visible (off-screen, minimised or hidden all report `Occluded`), so those runs
draw unpaced; plates never depend on presenting.

## Timing a scene

`--bench <dir>` is native's half of the [frame-rate
comparison](../../web/src/battle/benchmark/README.md#the-frame-rate-comparison).
It draws every frame record of the scene back to back through the whole pass
list, framing each one as a plate is framed, and writes no plates. Each frame
records the same things as the browser's reel: its interval, its CPU time, and
its GPU time from the first timestamp marker pass's start to the last one's
end, as the browser takes it. The CPU time is split into
framing, preparing, encoding and submitting; submitting includes acquiring
the window's surface, copying the frame to it and presenting it. The report goes to
`native-<scene>-<time>.json` in the browser file's shape. A window that isn't
shown presents nothing and never blocks, so the bench keeps at most two frames
on the GPU, as a swap chain would. Every frame is drawn, so the static batches
are written only when they change. Run it on a release build, under the GPU
lock.

## Checks

`tests/pipelines.rs` builds every exported pipeline on this machine's device.
`tests/counts.rs` replays a committed count-only browser run (`fixtures/parity/capture/counts`,
gzipped as written) and holds native's counts (`counts.rs`: the models, the
effects, the building modules, the props and the scenery, all the
presentation's) to the browser's on every frame it counted, on tick boundaries
and between; it needs no GPU. A replay check of a plate counts the same way.
The others replay the fixture run's world through what native reads (the
models, the props, the scenery and the sound) and need the Git LFS files each
names. Plates are judged against the control's, region by region, with the
spec's screenshot skills.
