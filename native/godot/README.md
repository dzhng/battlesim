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
  blocker and exits, so a stall frees the GPU lock on its own.

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

- **Pacing.** Uncapped by default: vsync off and no frame-rate limit, so a
  frame takes as long as it costs. `--pace display` turns vsync on instead,
  for comparing with a browser whose frames the display paces.
- **Presentation.** A bench runs minimised, like every run, and a minimised
  window doesn't present its frames, as the headless browser it is compared
  with doesn't. Presenting is a separate cost: the same 300 market-town
  records took 25–26 ms a frame minimised and 37–48 ms with the window shown
  (the project's `window/size/mode` at 0).
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
  formula: light and atmosphere, the ground, the grass field.
- `world/` holds what native still draws its own way, each piece reported as
  an approximation until a later slice ports its browser owner.
- `reel/` places the camera, steps the records and writes plates; `report/`
  owns the report and the error logger; `assets/` stages repository assets.
- `core/` holds the concepts more than one of those needs, each with one
  owner: the coordinate conversion, word tables, strict JSON checks, numbers
  written into shader text, and compute passes. A concept's shader-text half
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
- **Sky light.** The ambient and reflected light is the ported physical sky
  times the fill: the sky shader writes that product into the radiance
  cubemap Godot lights with, whether or not the sky is shown.
- **Sky.** The shown sky is the same physical sky, with the horizon's
  radiance below the horizon, the sun's disc and the cloud layer over it.
- **Samples.** The world is drawn at the browser's 4× MSAA, and both effects
  below read Godot's resolved colour (and depth), as the browser's post reads
  its resolve.
- **Haze.** A compositor effect hazes every surface once, from the depth
  buffer, toward the frame record's ground focus, before the post chain. It
  evaluates the sky radiance directly where the browser samples its sky LUT.
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
  browser's, or the run stops at a blocker. The browser leaves clumps out on
  the side's learned scars (craters, scorch and tracks) and lays them flat;
  native replays those scars (`capture/learned_ground.gd`) but doesn't draw
  them yet. So a cell native overgrows by no more than its clumps on such a
  scar is a `scar-omission` budget row instead.
- **Reruns.** The build appends clumps in whatever order the GPU finishes
  them, so two runs of the same plate can differ in a few pixels where blades
  overlap. Compare plates against that floor, not against zero.

## Legacy map layer

Until phase 2 ports the browser's model owners, the map is drawn roughly:
- building shells picked from a catalog kit by template token;
- catalog props or boxes;
- units as unposed catalog models at their authored size, or capsules.

Every category is reported as an approximation, and rows farther than 1 km
from every frame target are reported as dropped.

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
  offline `native_binding_probe` in a scratch project. It has its own
  `.gdignore`d folder, so this project never loads the binding:
  `binding_smoke/binding_smoke.sh <godot> <scenario.json>`.
