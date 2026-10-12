# Native Godot client

A second client beside the browser. Godot draws what the browser recorded,
and Rust stays the only authority. It renders a browser capture run frame by
frame, so the two clients can be compared side by side under the same frame
records. The plan it serves is
[`specs/native-rendering-parity`](../../specs/native-rendering-parity/README.md).

The project renders with Forward+ on Metal only. Only macOS is verified.

## The parity run

One run renders one scene that the browser's capture run wrote:

```text
lockf -k <main checkout>/throwaway/gpu.lock \
  godot --path native/godot -- \
  --run <dir> --scene <name> [--frames plates|all|<i,j,...>] [--maps <dir>] [--out <dir>]
  [--bench <warm-up frames> [--pace uncapped|display]]
```

- **Input.** `<dir>/<name>/` holds the browser's `capture.json`
  (`battle-presentation-capture`) and `manifest.json`
  (`native-visual-presentation`), with the manifest's hashed table artifacts
  beside them. The map comes from `--maps`, by default `fixtures/maps`.
  Relative paths resolve from the repository root.
- **Identity.** The run refuses a battle whose capture or map bytes don't
  match the manifest's sha256, or whose scene, map, encounter or seed differ
  from the capture's.
- **Stepping.** The run steps frame records by index: `plates` (the records the
  browser shot), `all`, or a list. There is no wall clock. Each record places
  the camera at its eye, looking at its target with its screen up, through the
  manifest's lens. It shows the latest publication at or before its tick.
- **Output.** For each record, the run writes `plate-<index>.png`, exactly the
  record's canvas. It also writes a `godot-render-report`. Both go to `--out`,
  by default `<dir>/<name>/native/`, which belongs in scratch.
- **Report.** It names every blocker that stopped the run, including any
  error Godot logged: a GDScript runtime error returns null to its caller and
  lets the run go on, so the run collects errors through a logger. Each budget
  row gives the owner, the reason, the source, and the counts admitted,
  rendered and dropped. A plate row carries its measured GPU time, which is
  null on Metal: Godot's Metal driver records no GPU timestamps. A bench run's
  report has no plates and a `bench` instead.
- **Exit codes.** 0 means no blocker, 1 means a blocker, 2 means bad arguments.
- **Time limit.** A run still going after 20 minutes stops at a `timeout`
  blocker and exits, so a stall frees the GPU lock on its own. A watchdog
  thread keeps the time, so a main thread stuck inside the engine is stopped
  too.

Rendering needs a display. `--headless` uses Godot's dummy renderer, which
never draws, so a headless run stops at a `display` blocker. The project
starts its window minimised, out of view; it needs a logged-in macOS
session. macOS clamps a window placed off screen (`--window-position`) back
onto the screen, so that doesn't hide it.

The run draws every frame itself (`RenderingServer.force_draw`, with Godot's
render loop off). Godot's main loop skips drawing, and sleeps, while its
window is minimised or covered, so a run that waited on it stalled for as
long as the window stayed hidden. Each record still passes one main-loop
iteration, which pushes moved nodes to the renderer; the low-processor sleep
is zero, so that iteration doesn't sleep.

## The frame-rate bench

`--bench <warm-up frames>` turns the same run into native's half of the
frame-rate comparison with the browser (the browser's half is the
[`graphics-fps` scene](../../web/src/battle/benchmark/README.md#the-frame-rate-comparison)).
It draws every frame record (or the `--frames` given) once, in order, exactly
as a plate run composes and steps it, at the same 4× MSAA, but reads nothing
back and writes no PNG.

- **Pacing.** Uncapped by default: vsync off and no frame-rate limit. Even
  so, frames cheaper than the display's refresh interval settle near it
  (8.3 ms at 120 Hz), on screen or off; below it, compare GPU time from a
  trace. `--pace display` turns vsync on instead, for comparing with a
  browser whose frames the display paces.
- **Presentation.** A bench presents nothing, as the headless browser it is
  compared with doesn't. The project starts minimised, but macOS can bring
  the window back on screen before the timing starts, so the bench
  minimises it again just before timing; the report's `window` says what it
  was at the end. On screen, presenting costs about 0.1 ms a frame, which at
  the display's step is enough to move a median.
- **Timing.** Each frame's interval runs from the end of one frame's draw to
  the next (`Time.get_ticks_usec` after each `force_draw`), and its CPU time is
  the record's step plus Godot's frame setup and viewport render on the CPU.
  The warm-up frames are drawn but not timed; pass the browser's count (60).
- **Report.** The report's `bench` holds those raw per-frame intervals and
  CPU times (`report/frame_timing.gd`), what the timed frames drew by
  Godot's own count (draw calls, objects, primitives), and the conditions:
  power source, adapter, refresh rate and window. GPU ms is null, because
  Godot's Metal driver records no GPU timestamps. Native computes no frame
  rate itself: the browser's summariser does, for both clients. By default
  the report goes to `<dir>/<name>/native-bench/`.
- **What it is not.** Native replays recorded publications with no simulation,
  and draws only the layers it has ported, so its numbers are a lower bound
  on a finished native client's cost, never a like-for-like verdict.

## The release build

Native is measured as it would ship: a release export of this project, not
the editor binary's debug run. `release.sh` exports the `macOS release`
preset (`export_presets.cfg`) into `throwaway/native-release/` and runs the
exported app off screen with the parity run's own arguments:

```text
lockf -k <main checkout>/throwaway/gpu.lock \
  native/godot/release.sh --run <dir> --scene <name> [any parity run argument]
```

- **Templates.** The export needs Godot's 4.7.2 export templates in
  `~/Library/Application Support/Godot/export_templates/4.7.2.stable/`
  (the official `Godot_v4.7.2-stable_export_templates.tpz`; only
  `macos.zip` and `version.txt` are used).
- **Preset.** A release template with no signing beyond Godot's ad-hoc
  signature, and the project's own rendering settings. The template ships
  only universal binaries, and an arm64 export needs ETC2/ASTC texture
  imports, so the project imports those beside S3TC; the editor still draws
  with S3TC. The grass build's `.compute` text, which the run reads as a
  file, is packed as one; the tests are left out.
- **Paths.** The app lives outside the repository, so it takes the root from
  `BATTLEGAME_REPO`, which the script sets; repo-relative arguments and
  assets resolve from it as in the editor.
- **Assets.** An export can neither link nor import. It packs what `staging/`
  held when it was exported, and an asset it lacks is a named blocker. An
  editor run of a scene stages its assets.
- **Report.** `renderer.build` says what ran the project: `editor`, `debug` or
  `release`.

## Layout

Each folder owns one kind of thing; `parity_run.gd` is the entry point, and
composes the layers the manifest lists.

- `capture/` reads the browser's files strictly (no defaults, no unknown
  fields, no stale shapes) and replays what the capture implies, such as the
  side's learned ground.
- `environment/`, `terrain/` and `grass/` each port one browser owner by
  formula: light and atmosphere, the ground, the grass field; `units/` ports
  the browser's posing of soldiers and vehicles, and `effects/` its combat
  effects.
- `world/` draws the map's models from the browser's placement: the
  instanced batches every model goes through, the buildings, scenery and
  props, and the fog overlay.
- `reel/` places the camera, steps the records and writes plates; `report/`
  owns the report and the error logger; `assets/` stages repository assets.
- `core/` holds the concepts more than one of those needs, each with one
  owner: the coordinate conversion, word tables, strict JSON checks, numbers
  written into shader text, compute passes, the browser's seeded sequence and
  text as JavaScript writes and reads it (`js_text.gd`). A concept's shader-text half
  sits beside it as a `.gdshaderinc`.

## Coordinates and camera

The sim is Z-up and right-handed; Godot is Y-up and right-handed. One
conversion, `core/coords.gd`, maps sim `(x, y, z)` to Godot `(x, z, −y)`, and
`core/coords.gdshaderinc` is the same conversion for shaders. It is a
rotation, not a mirror. Every placement, the sun and the camera go through it,
and a sim yaw becomes the same angle about Godot's +y.

The camera takes the browser's vertical field of view (`KEEP_HEIGHT`) and near
plane from the manifest's lens. Godot has no infinite far plane, so the far
plane is the map's diagonal plus the backdrop's reach from the manifest's
light, and the report records it as a budget.

## Publication

Native decodes the capture's publications with the same Rust reader the
browser uses (`crates/sim/src/publication/reader.rs`), through the extension's
`PublicationReader`: one ordered stream, each sample one observation with the
browser's keys and nulls, the fog bits and the ground runs (with their cleared
byte). An unchanged static group is the same `Array` as the sample before. A
layout that isn't the writer's, a short payload, a bad encoding or a record
that breaks its baseline stops the run at a `publication` blocker.

A record's `simTime` is read as the browser reads it, to the nearest double
(`core/js_text.gd`): Godot's own JSON reader can land an ulp off, and a clock
exactly on a tick's boundary then falls on the other side of it.

The reader's check is `crates/sim/tests/publication_reader.rs`: every sample
of the fixture captures equals `tests/fixtures/publication/<scene>-observations.json`,
the browser's last decode before the reader replaced it (the fog words by
digest). The market-town fixture is seven samples of a control capture whose
first record is re-encoded as a fresh one, for the groups paris-corner never
publishes.

## Light

The manifest's `light` is the browser's `presentation.light`, exactly as the
battle was drawn with it, plus the unit `sunDirection`. `environment/light.gd`
reads it strictly: every field, nothing extra and no defaults. Native ports
only the browser's formulas (`packages/battle-renderer/src/light/` and its
shaders) and writes the manifest's numbers into them:

- **Sun.** Its colour is the sky model's transmittance toward the sun, its
  intensity the fixture's, and a shadowed surface keeps `shadow_floor` of it.
  Godot's directional energy differs from the browser's radiance by a factor
  of pi, which the adapter divides out.
- **Cascades.** Each frame the four splits run over the part of the ground
  the camera sees, as the browser's do. Godot has one penumbra radius per
  light in each split's own texels, so the penumbra matches the browser's in
  the second cascade and widens beyond it.
- **Sky lookup.** The clear sky's radiance is integrated once, into the
  browser's 384×192 equirectangular lookup (`environment/sky_lut.gd`), and
  everything below reads it with linear filtering, as the browser's sky,
  environment light and haze read its LUT.
- **Sky light.** The ambient and reflected light is the looked-up sky times
  the fill: the sky shader writes that product into the radiance
  cubemap Godot lights with, whether or not the sky is shown.
- **Sky.** The shown sky is the looked-up sky, with the horizon's
  radiance below the horizon, the sun's disc and the cloud layer over it.
- **Samples.** The world is drawn at the browser's 4× MSAA, and both effects
  below read Godot's resolved colour (and depth), as the browser's post reads
  its resolve.
- **Haze.** A compositor effect hazes every surface once, from the depth
  buffer, toward the frame record's ground focus, before the post chain.
- **Tone.** A second compositor effect is the browser's post chain: bloom,
  grade and AgX at the manifest's exposure. Godot's tone mapper is linear and
  its glow off, so its output step only adds the sRGB transfer the browser's
  post ends with.

## Layers

Only the layers the manifest lists are drawn, under the browser's layer names.
A listed layer that native can't draw is reported as dropped. Fog is decoded
from every publication but painted only when `fog` is listed.

## Assets

The run links each repository asset it draws, one file at a time, into the
ignored `staging/` folder. It then imports whatever is new with one
`godot --import`. Linking a directory instead would make Godot write `.import`
files into the real `assets/` tree.

A missing file, a Git LFS pointer or a failed import is a named blocker, never
a stand-in. Fetch what a scene needs with `git lfs pull --include=<path>`.

## Word tables

Native draws the ground and grass from the browser's own tables, not from the
map: the manifest's hashed artifacts are the exact words the browser's GPU
read. `core/word_tables.gd` uploads them. Godot 4.7 can't wrap an `R32_UINT`
device texture, so each word is one RGBA8 texel, and
`core/word_tables.gdshaderinc` puts it back together exactly. A one-record
`params` table is uniform vec4s instead.

Each shader compiles with defines generated from the manifest: each table
field's offset from its layout, and every number from `tuning`
(`core/shader_text.gd` writes them so they read back as the same f32). A
layout or tuning change reaches native without a code change, and a renamed
field fails to compile, which is a logged blocker.

## Terrain

- **Height.** `terrain/height_field.gd` reads the paged height grid with the
  simulation's triangle rule. The ground mesh, and everything that stands on the
  ground without a height of its own, seats on it.
- **Formulas.** `terrain/ground.gdshaderinc` is the browser's
  `terrainMaterial.ts`, ported as text. Numbers that are part of a formula
  (hashes, lattice turns, decorrelation offsets) stay in the text, as they do
  in the browser.
- **Scars.** The side's learned ground (`capture/learned_ground.gd`) is
  replayed record by record and kept as the browser's scar bytes per cell in
  one word table (`terrain/scar_texture.gd`): the browser's normalisation
  words, a directory word per 16-cell tile and a page per marked tile. Only
  changed tiles are rewritten. The ground reads it with the browser's exact
  RGBA8 filter arithmetic and cuts craters, scorch, tracks and trampling as
  the browser does; the biome's scar look is the manifest's terrain table
  `scars`. The browser's view-sized page cache isn't ported: the whole learned
  ground fits.
- **Water.** With the `water` layer, the water surface is drawn over the
  browser's water cells (terrain table `water`), blended over the ground and
  reading depth. Neither reel map has water, so the surface formulas are
  checked against the browser's WGSL on WebGPU
  (`tests/water_vectors_display_test.gd`), not on a plate. Native keeps
  Godot's own light on water in shadow rather than the browser's
  `water.shadow` share of it, a reported approximation.

## Grass

Native grows the browser's grass field the way the browser does, from the
browser's own tables (`manifest.grass`), anew for every frame record.

- **Build.** `grass/grass_build.compute` is the browser's build ported as text
  to a GLSL compute shader. It includes the ground formulas
  (`terrain/ground.gdshaderinc`), whose uniforms it rebinds to compute bindings.
  Each frame's view fields (side planes, eye, window, tiles) are recomputed
  from the record; the rest of the browser's params are used as written.
- **Tiers.** Each tier (near, far) is a MultiMesh whose buffer the build writes
  and whose indirect draw command it counts. This mirrors the browser's clump
  buffer and indirect args. A compute-written MultiMesh renders on 4.7.2 as
  long as it has a custom AABB. Godot leaks an indirect MultiMesh's command
  buffer when the MultiMesh is freed, which shows as a warning at exit.
- **Draw.** `grass/grass.gdshader` is the browser's blade vertex and fragment.
  It places each blade vertex of the clump's kind from the browser's shape
  table, and bends it in the one wind on the record's interpolated sim time.
  A far clump's tint and light give way to the ground's.
- **Agreement.** The hash words, tile offsets and admission decisions equal the
  browser's WGSL on WebGPU exactly. The two GPU compilers round the fall
  point's arithmetic differently, by a few ulps.
- **Counts.** For each plate the report puts native's clumps per screen cell,
  counted as the browser counts them (root on screen, occlusion ignored),
  beside the browser's `grassCounts`. Every cell must be within 10% of the
  browser's, or the run stops at a blocker.
- **Scars.** The build reads the ground's scars: craters, scorch and tracks
  leave clumps out, and tracks and trampling lay them over, as the browser's
  do.
- **Reruns.** The build appends clumps in whatever order the GPU finishes
  them, so two runs of the same plate can differ in a few pixels where blades
  overlap. Compare plates against that floor, not against zero.

## Models

`world/model_batches.gd` draws placed models as the browser's models layer
does: instanced batches, so draw calls follow appearances × tiers ×
materials × passes, never placed objects or their parts.

- **Appearances.** Any node tree of meshes. Each tier's meshes merge into one
  mesh with a surface per material, every part's transform baked in. A mesh
  names its tier with a `_LOD<n>` suffix (finest first, the scene assets'
  rule) and an unsuffixed mesh is in every tier. A skinned mesh is the units
  layer's (below). Godot's own simplified meshes are off
  (`project.godot`): the authored tiers are the one level of detail.
- **Surfaces.** Every material draws as the browser's model surface
  (`world/model_surface.gd`): base colour times vertex colour, its colour
  scale and texture, and the instance's tint by the material's share (a
  scenery appearance takes it whole). Each instance carries its tint.
- **Chosen.** Each layer ports its browser owner's rule for which model
  draws at which tier and which casts (buildings, scenery, props, units), and
  hands each appearance and tier the instances it draws into the view and
  those it casts (`choose`). A view batch casts nothing; a caster batch draws
  only into the cascades. A batch's buffer is written only when its layer
  chose anew, and each layer chooses anew only when what it draws changed.
- **Culling.** Layers cull and tier against `world/detail_view.gd` (the
  view's side planes, pixels per metre and shadow reach), chunk by chunk.
  The static world's walk (buildings, scenery, props) runs in the Rust
  extension (below); the units layer's stays in GDScript.
- **Casters by region.** A static layer's casters are batched per 512 m
  square region, so each cascade draws only the regions its bounds meet.

## The Rust extension

The project loads the Rust GDExtension (`native/godot-binding`, descriptor
`binding/battle_binding.gdextension`), the native core, for the per-frame
work. `binding/build.sh` builds the release library into `binding/` (build
output, never committed); `tests/run.sh` and `release.sh` run it, and an
editor run needs it run first whenever the crate changed.

- **`BuildingWalk`** (`building_walk.rs`) is the browser's own buildings
  walk (`crates/static-world`): it resolves the capture's saved map against
  the runtime catalog's template art, takes what the side knows and the view,
  and hands back each module and tier's records as MultiMesh rows.
- **`StaticLayer`** (`static_layers`) walks the trees and shrubs and the
  props each frame. A layer hands it the browser's word tables once per map,
  what the side knows whenever that changes, and
  the view every frame (`detail_view.gd` `packed`), and gets back the kinds
  and tiers whose drawn or cast instances changed, with those instances, for
  its batches (`model_batches.gd`, which the units layer shares). The forest
  floor's dressing, a few dozen cells a frame, stays in GDScript.

The layers' tests (`tests/*_layer_test.gd`) and the plate counts test the
rules through it; `cargo test -p godot-binding` the casting the layers do
not reach.

## Buildings, scenery and props

Rust resolves the buildings for both clients (`crates/static-world`), and the
browser exports the scenery and props, static per map (`manifest.scenery` and
`.props`); native places nothing, and ports the per-frame rules and the side's
knowledge. `world/static_world.gd` builds the layers the
manifest lists and counts each plate.

- **Buildings** (`world/building_layer.gd`, the browser's
  `buildingPlacements.ts`): every building's kit module instances in each
  state. A 64 m chunk in view draws its coarse rows, or its rows at the tier a
  metre's pixels at its nearest point call for while the pool has room,
  nearest first. A building the side knows destroyed draws its ruin's or
  gutted shell's rows, by what its parts became (`knownProps`).
- **Scenery** (`world/scenery_layer.gd`, `sceneryLayer.ts`): the forest, its
  understorey and the backdrop past the map in 128 m chunks, a far chunk whole
  at the coarsest tier and a near one tree by tree; the forest floor's dressing
  a cell at a time. A tree the side saw fall (`fallenBodies`) falls from
  `(tick − 1) / tickHz` and lies by its stump (`world/felled.gd`, posed in
  the model surface's vertex stage as the browser poses it); a shrub or a piece of dressing on ground it saw cleared is gone.
- **Props** (`world/prop_layer.gd`, `propAppearance.ts`): the map's props as
  the browser fitted them, less those the side knows replaced, and every prop
  it learned (wrecks, shoved bodies, rubble) fitted here from the browser's
  candidates. As the browser's models layer, a 64 m chunk in view is visited
  and each prop in it culled and tiered on its own, cast a tier coarser.
- **Appearances** resolve through `world/appearance_catalog.gd`: a kit's
  module, or an appearance's state, staged from the asset catalog.
- **Counts.** For each plate the report puts native's building modules per
  kit module and tier, scenery per population, appearance and tier, and known
  props per appearance and tier beside the browser's `instanceCounts`; any
  difference is an `instances` blocker.
- **Settled at once.** The browser expands residents and lays dressing over
  a few frames after a cut; native settles each frame whole, which is what
  the browser shows mid-shot. A view whose residents would pass what the
  browser's pool packing holds is a blocker.
- **Not ported:** the leaf clumps' shading, the dressing's shrink below a
  few pixels, wear chips and lit interiors (reported as approximations).

## Units

`units/` draws every soldier, vehicle and corpse the browser poses, posed as
the browser poses them, through the model batches.

- **Poses.** The browser's per-frame derivations are ported line for line,
  on the frame record's clock: `tick_interpolator.gd` blends the record's
  publication from the one before (`interpolate.ts`), `pose_feed.gd` reads it
  as the pose driver does (`poseFeed.ts`, with `effects/launches.gd` naming which
  soldier fired), and `pose_driver.gd` is the driver (`poseDriver.ts`): gait,
  facing, stance and lean, deaths and the corpse cap, turret bearings, eased
  elevations, recoil, track travel and an aircraft's lean. Poses carry from
  frame to frame, so every record is posed in order, drawn or not.
- **What draws them.** The manifest's `units` (`capture/unit_manifest.gd`) is
  the browser's own resolution: each fielded type's facts and the appearance
  sets `AppearanceCatalog` picks from per side, the pose feel, and each
  appearance's joints and clips (a body, its clip table a hashed artifact) or
  nodes and rig (a vehicle). `unit_poses.gd` picks a soldier's member by id
  from them.
- **Soldiers and corpses** keep the glTF's own vertices and skin. Each skin
  bind moves with the bundle joint of its name: the GPU pose kernel
  (`pose_kernel.gd`, the browser's `poseKernel.ts`) samples the clip table and
  writes one matrix per bind into a palette image, and the model surface skins
  each vertex from it (`POSED_SKINNED`). A corpse is the same body at its
  corpse pose, one palette per body and way of lying.
- **Vehicles** keep their glTF parts, each following the bundle node it hangs
  from: the articulated nodes are posed on the CPU (`articulation.gd`, the
  browser's `articulate`) into a second palette (`POSED_RIGID`), and a track's
  texture scrolls by its links' travel.
- **Detail.** Each model takes the browser's rule (`modelDetail`): culled
  off screen, else the tier its projected height calls for, its shadow cast
  one tier coarser. Not yet: the browser's impostor cards for models under
  `impostor_px`, which draw their coarsest tier; the report counts them.
- **Checks.** The build refuses a glTF whose joints or nodes do not stand
  where the bundle's do. Each plate's posed models by kind (`modelCounts`)
  must equal the browser's, or the run stops at a `models` blocker.

## Effects

`effects/` draws the browser's combat effects from the causes the capture
already publishes, on the frame record's clock and with the browser's seeds.

- **Causes.** Every publication up to a record's tick is read as the browser
  reads it for effects (`effect_feed.gd`, `effectFeed.ts`): visible flight,
  blasts, each seen unit's shot counters and hull, the airframes falling, and
  the wrecks and ruins the side knows. `cook_offs.gd` finds the hulls the side
  watched brew up. `launches.gd` names who fired and which rounds show as
  tracers; the pose feed reads the same tracker.
- **Effects.** `effect_frame.gd` is the browser's `EffectFrame` line for line:
  tracers and smoke trails, flashes, impacts, sparks, blasts, dust, smoke from
  smoking hulls, flames and smoke over wrecks and ruins, cook-offs and rotor
  blurs, with their cast lights. Per tick it seeds from
  `mulberry32((tick·2654435761)>>>0)`, consumed in publication order; dust,
  smoking hulls and smoke sources seed from the browser's `hashString` keys,
  written as JavaScript writes their numbers. The manifest's `effects` holds
  the tuning and the catalog rows it reads.
- **Placement.** A flash sits on the muzzle the units layer drew
  (`drawn_models.gd`, `drawnModels.ts`): a vehicle's muzzle node under its
  articulation, a soldier's weapon socket in his clip.
- **Drawing.** `effect_pass.gd` is the browser's effect pass as a raster pass
  between the haze and the post chain: streaks, glows and discs, flipbooks
  from the effect atlas, rotor blurs, tested against the world's depth.
  Lit smoke takes the sun, the sky (averaged over each hemisphere, a reported
  approximation of the browser's prefiltered environment) and the cast lights.
- **Cast lights.** `cast_lights.gd` keeps the lights in view, the strongest
  past 24, and hands them to every world material through shader globals
  (`cast_lights.gdshaderinc`, added to emission as the browser adds them).
- **Counts.** For each plate the report puts native's effects running by kind,
  instances by shape and lights beside the browser's `effectCounts`; any
  difference is an `effects` blocker.
- **Not drawn:** a cook-off's moving wreck and the debris it throws (the
  static wreck shows throughout), reported per run.

## Checks

- `native/godot/tests/run.sh [tests/<name>_test.gd ...]` runs the tests:
  headless, except a `*_display_test.gd`, which renders in an off-screen
  window, so take the GPU lock around the run. A test passes only when Godot
  exits 0, prints its result line and logs no script error, because a parse
  error also exits 0 and a runtime error in a called function doesn't stop
  the test. Script-mode tests can't use `assert`, because a failing assert
  hangs the run.
- The fixture run is the runner's display-backed check. It renders the
  real browser capture under `tests/fixtures/`, trimmed to paris-corner up to
  its first plate frame, with a frozen copy of its map:
  `--run native/godot/tests/fixtures/run --scene paris-corner --maps native/godot/tests/fixtures/maps --out throwaway/native-runner/fixture`.
  A trimmed capture changes its bytes, so its manifest's `captureHash` is
  recomputed.
- `tests/forward_plus_probe.gd` is the backend's display-backed probe. It
  renders a real market-town building from its staged kit, the grass build's
  MultiMeshes, a shadowed sun and the sky. Run it with
  `godot --path native/godot --window-position 10000,10000 --script res://tests/forward_plus_probe.gd`.
  It needs `assets/source/city/homes/kit.glb` from Git LFS.
- The [binding smoke](binding_smoke/) checks the Rust GDExtension against the
  offline `native_binding_probe` in a scratch project, from its own
  `.gdignore`d folder and a debug build:
  `binding_smoke/binding_smoke.sh <godot> <scenario.json>`.
