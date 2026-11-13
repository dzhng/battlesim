# Tracked armour models

Every tracked armoured unit (37 appearances: the tanks, heavy and light IFVs,
recon vehicles, self-propelled howitzers, and tracked air defence and rocket
launchers) wears a body that reads as its real vehicle at the zoomed battle
camera and in cinematics. Each body is a standalone Blender script,
`build.py`, written by Gemini 4 Argon in a Bento session from the reference
photos and our measurements, then edited locally; the family's export script
wears it through `scripted_body`.

The lineup the pass ended on, at one scale in the game renderer:

![The 37 tracked models at the battle camera](assets/after/lineup-battle-near.png)

[`visualizations/lineup.html`](visualizations/lineup.html) sets each one beside
its reference photo and the model it replaced.

## Why it took this shape

The pass began as hand-written procedural geometry, shared parts and a frame
survey (the shared slices: the lint, the vehicle-parts split, the gun tube,
skirts and outlines, track sag, plate edges, closed hatches, frame fixes).
Measured against the photos it stayed toy-like, and two pivots changed the
route:

- **Generators (G1–G6).** Image-to-3D generators fed matched, photoreal views
  of the measured model came back with the inputs' shape errors in every
  output and the wrong proportions (Tripo multiview about 15 % short), so a
  generator can't be the source of proportions.
- **Argon (A1–A6).** The user's call: Argon builds each model as a Blender
  script. A script is code, so it is measured, linted, diffed and edited like
  any source, and its own renders let it iterate against the photos. Two
  rounds per model (to save money), then local edits. A third round went
  where the user asked for one (the Leopards' turret, S1) and where a turret
  was beyond local edits (the Challenger 3's, S2).

Proportions stay the coordinator's job: every round is measured against the
battle frame (`fixtures/catalog.json`), which the art never moves (P9). The
frames changed once, in slice 15, with named digest changes; everything after
is art, moving no frame, mount or rule, so it leaves digests and replays alone
(T1).

## Contracts

- **A vehicle's body is its `build.py`** under
  `assets/source/skins/<family>/<round>/`. Its `build()` (or `build(variant)`)
  builds into the current scene and names parts by what moves them (`hull`,
  `turret`, `gun`, `hmg`, `hmg_gun`, `wheel_*`, `track_L`/`_R`, `dressing_*`),
  painting with seven slots (`armor_paint`, `fittings`, `steel`, `rubber`,
  `optics`, `lamps`, `recesses`). It also runs alone, which is how Argon
  iterates and how the next round starts. The README of the round a family
  wears lists its rounds and local edits.
- **`scripted_body.wear`** fits a script's output to the export contract: it
  runs the script, moves each part under the rig node of what moves it (the
  rig is the frame's), keeps its world place, makes one object per material
  slot in the family's paint, replaces the script's track runs with the
  shared belt (`running_gear.track_loop`) and builds the tiers.
  `scripted_body.wreck` makes the wreck, keeping Trophy's housings;
  `scripted_body.trophy_nodes` places the Trophy part's nodes on a body with
  one housing a side (the M1E3, with a housing per part, places its own).
- **A family script** holds only the body's path, the link pitch, the
  trunnion, the rig, the whips (a mirrored pair is `roof_parts.whip_pair`)
  and any node the game needs, the wreck, and a docstring of what the photos settle and the frame
  it is built to. It never draws body geometry beside a worn body.
- **Unchanged:** the renderer, the GLB node contract (`track_L/R`,
  `wheel_*`, `track_length_m`, `link_pitch_m`, mount nodes, `dressing_*`,
  `_LOD0..3`), the bundle formats and the simulation.

## Principles that must keep holding

- **Silhouette and proportions first,** judged where the player sees them:
  the zoomed battle camera and cinematics. Small fittings only where they
  read there.
- **One clean factory scheme per nation:** US ground units desert tan, the
  European, Russian and Chinese ones green woodland (the user, S1). Paint is
  matte (R13: clean roughness 0.8 on the sprayed schemes, 0.74 on the German
  three-tone), and hardware is a darker shade of the scheme's own paint
  (`vehicle_export.FITTINGS_SHADE`, S5): a fixed brown read tan on green.
- **A turret's front is swept back** in plan, from the mantlet to each side,
  at least 0.5 m back per metre outboard; never flat across and never angled
  in toward the gun (the user's rule, R12). Weapons standing at the edge (the
  T-15's missiles, the BMP-2M's grenade launcher) are judged by the armour
  body. A new front is built swept into the parts: a sweep pushed after the
  build flattens what it clamps and reads as a box with a wedge stuck on (the
  Leopard, S1). Four bodies still sweep after the build, where it reads well
  (the KF51, Challenger 2 TES, Leclerc and M10; S4).
- **Hatches closed, no crew** on any tracked model.
- **Every tracked body lints clean** (P10); none is in the lint baseline. A
  fix goes in the body's `build.py`, never in the exported file.
- **Every receipt matches its sources** after a shared change: a change to
  shared code re-exports every family it reaches (S11 left only the infantry
  kits, which have their own exporter).
- **A far tier never reaches past the near one:** `mesh_lods` sheds a
  family's smallest pieces when their floors overrun the budget (R11) and
  keeps each decimated piece inside its source box (S7).
- **A wreck may differ from its box by 1 m on any face** (the scenery kind's
  `rubble_m`; the user, R10): rubble spreads and a heaved turret stands
  proud.

## Pointers into the code

- `packages/scene-assets/blender/scripted_body.py`: `run_script`, `wear`,
  `wreck`, `trophy_nodes`.
- `packages/scene-assets/blender/roster/*.py`: one script per family; three
  cover several families: `sp_howitzers.py`, `rocket_artillery.py` (the
  M270/MARS II on bodies, HIMARS, BM-21 and Tornado-S procedural) and
  `air_defence.py` (the Gepard and Buk on bodies, the wheeled cards
  procedural).
- `packages/scene-assets/blender/mesh_lods.py` (`reduced_copy`),
  `vehicle_export.py` (`materials`, `FITTINGS_SHADE`, the receipt),
  `textures.py` (`_sprayed` and `nato_camo`, the matte schemes).
- `packages/scene-assets/blender/vehicle_lint*.py` and
  `vehicle_lint_baseline.json`; `packages/scene-assets/src/boxFit.ts`
  (`footprintFindings`, the rubble floor) and `scenery.ts` (`rubble_m`).
- Tests: `running_gear_test.py` (the belt), `wreckage_test.py`,
  `vehicle_lint_test.py`, `vehicle_lint_logic_test.py`, run by `asset check`;
  `web/scenes/unit-roster.mjs` and its baselines; `web/tutorial-shots.mjs`.
- [`packages/scene-assets/blender/README.md`](../../../packages/scene-assets/blender/README.md)
  owns the commands (export, lint, sheet, bake, check).

## How a body is made

The brief Argon gets is [`argon-brief.md`](argon-brief.md): a common part sent
verbatim (the script contract, slots, clearances, shape rules) and a vehicle
part from the frame. The recipe the pilot proved is in
[`family-logs.md`](family-logs.md#the-argon-pilot-t-72b3-and-abrams-sep-v2):

1. One session per family; check its first turn ran Argon (`usageByModel`;
   A5).
2. Round 1 builds. Measure its GLB against the frame and run the export once,
   so round 2 gets measured fixes and lint pairs, never more than about ten.
3. Round 2 fixes them. Then local edits in `build.py`, each listed in the
   round's README with its before and after numbers.
4. Export, lint, validate, critique against the references, then bake.

A Bento turn can end with an empty reply and no files ("finish_reason: other"
or a stream error, while the session reads complete); "continue" in the same
session finishes it, as the same round.

## Dead ends

- **Procedural tanks** (slices 00–15): the shared parts made consistent but
  toy-like models; the code is deleted now that every body is a script (S9).
- **Image-to-3D bodies** (G1–G6): wrong proportions, and the inputs' errors
  in every output.
- **A cast turret from a generator** (slice 03) and **baked panel seams**
  (slice 04): the smoothed procedural dome and painted seams won; the T-72
  turret pin (`blender/skins/t72.json`) is that experiment's provenance.
- **A shared post-step for the swept front** (`sweep_turret_front`, B10):
  where it clamps a large front it flattens the wedges into an add-on; the
  Leopard's and Challenger 3's third rounds built the front swept instead.
  The KF51, Challenger 2, Leclerc and M10 keep their per-script copies (S4).
- **Rubble only outward** (T3): the wrecks' heaved turrets stand above their
  boxes, and the user asked for 1 m on the wreck check in general.

## Visual provenance

- [`assets/before/`](assets/before/): the planning lineup the user annotated,
  with the reference each model was judged against
  ([`refmap.json`](assets/before/refmap.json)) and its close-ups.
- `assets/references/<family>/` (repository root): the photos and drawings,
  with licences; each `references.json` notes what the frame survey measured.
- [`assets/frames/`](assets/frames/) and
  [`visualizations/frames.html`](visualizations/frames.html): the frame
  survey the battle frames were fixed from.
- [`assets/after/`](assets/after/): the compare sheets per family (the
  T-72B3's and Abrams SEP v2's under `argon-pilot/`), the shared slices'
  evidence (`08/` to `12/`), the lineup close-ups (`lineup/`), and [`assets/after/lineup-montage.png`](assets/after/lineup-montage.png)
  with the same close-ups as `assets/before/closeups/`.
- [`assets/references/jet-sheen-reference.png`](assets/references/jet-sheen-reference.png):
  the user's screenshot behind the jets' paint sheen (slice 05).
- [`visualizations/map.html`](visualizations/map.html): the planning walk's
  map, decision ledger and landmine cards.

## Below the bar

What the pass leaves short of the bar (each family's scores and detail are in
[`family-logs.md`](family-logs.md)):

- **Shared:** the paint's low-down dirt and rain streaks are vertex-painted,
  so they show on dense meshes (the three Abrams' smudges) and not on plain
  ones; thin bars (the T-14's slat cages) take the full edge chip (LA-S1);
  shaded sides read dark or olive under the workbench sun.
- **Frame-bound heights:** several turrets read low to critics because the
  frames hold their roofs: the ZTQ-15, ZTZ-99A, CV90120, Bradleys, Puma.
- **T-80BVM:** the Relikt tiles round its cast dome read as an older Kontakt
  turret.
- **T-14 and T-15:** the T-14's slab sides and slat cages; the T-15's skirts
  read as a tiled grid.
- **Challenger 2 TES:** a small commander's sight and thin side packs.
- **CV90 family:** small sights (the CV9040C's drum, the Mk IV's mast and
  Spike pod), a boxy nose.
- **Bradleys:** a small TOW box and 25 mm, glacis tiles that read as planks.
- **Ajax:** a tall near-vertical lower nose plate.
- **BMP-2M, BMP-3, BRM-3K:** plain turret detail; faceted shading on the
  BMP-2M's bow; the BMP-2M reads as a drum from the front quarter (the real
  turret's shape).

## Later passes

One per unit group, each with its own spec: wheeled armour (decide whether
"no crew" carries over, and with it `vehicle_crew.py` and `Vehicle.head_out`,
which the Centauro still uses), helicopters, wheeled artillery and air
defence, trucks, infantry (re-export the kits for R11 and S7), drones, jets
off the generator (slice 06: code-smoothed guides beat the generated skins),
and texture-space weathering, so dirt reads the same on every mesh.

The decisions, with who made each and why, are in [`decisions.md`](decisions.md).
