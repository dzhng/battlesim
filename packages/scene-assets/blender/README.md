# Appearance source authoring

These scripts make committed source art; they are not part of game startup or
ordinary bundle baking. [The asset CLI](../../../web/asset.mjs) launches headless
Blender on one thread and checks the pinned version: an export must repeat byte
for byte, and Blender's threaded evaluation does not. Set `BLENDER` to another installation path
when needed; that changes the executable location, not the accepted version.
Exporter arguments are the source script's own usage text and follow Blender's
argument separator through the CLI.

For one source, run from the repository root:

```bash
bun run --cwd web asset -- blender ../packages/scene-assets/blender/<script.py> <arguments>
```

The CLI resolves a script relative to its web working directory. An absolute
script path also works. Output destinations are the exporter's arguments or
subject-owned default, so inspect its usage before replacing committed art.

## Source families and dependencies

[Infantry clip export](clips_infantry.py) builds a hold family's animation library;
[infantry kit export](infantry_kit.py) builds bodies and equipment using that rig.
Each roster kit ([roster equipment](roster/infantry_equipment.py) `KITS`) names its
weapon and the armies it is built in (`ARMIES`, named as its reference variants):
each army's uniform print ([`textures.py`](textures.py) `UNIFORMS`), helmet,
carrier and rifle on the same rig. A kit's first army is its default look; the
others are the appearance's faction looks.
Clips and meshes must retain the shared skeleton contract: regenerating one does
not regenerate the other. Shared rig, weapon and mesh-tier helpers concentrate
that policy rather than defining extra standalone asset commands.

Visible vehicle crew ([crew module](vehicle_crew.py)) reuse their faction's
exported soldier art in fixed operating poses, from the soldier's tier 1 down
and without the weapon. They belong to the vehicle's appearance and mount
hierarchy, rather than adding simulation soldiers. Re-export their vehicles
after changing the infantry source; the crew exporter preserves that source's
exact textures and material masks on the crew's own materials only.

Reusable vehicle detail (wheels and their treads, whole tracked running gear, a
lofted hull's plan rings, bolted applique and armour tiles, slat armour,
hatches, cupolas, sights, lights, tow points, stowage, cargo beds, grilles,
exhausts, machine guns) has [one owner](vehicle_parts.py); a family script
places those parts rather than redefining them. A part hung on a hull or
turret side that leans takes its position and tilt from that face
(`on_side`), so plates, tiles, skirts and racks lie on the slope the photos
show instead of standing upright beside it. A family derived from another
(the T-90M, T-80BVM and Type 99 from the T-72) imports that family's script
for what they share.

No tank's hull is flat side-on: its top falls toward the nose over the
glacis, about 11 degrees along the fenders, and skirts, side armour and
fender boxes follow that line down. Where the photos show a centre upper
glacis plate steeper than the fenders beside it (the T-72's family and the
Type 99), it is its own plate between them, not one slope across the hull.
Skirts are thin plates whose inner faces stand clear of the track belt and
the road wheels; nothing in the bake or check catches a skirt cutting into a
track, so judge it in renders, including from the high battle camera.

A vehicle's surfaces come from the named helpers in [`parts.py`](parts.py),
each carrying its material role: `tyre()`, `glass()`, `track_steel()`,
`bare_steel()` and `paint(scheme)`. A family's scheme is its real nation's,
named as data in its exporter's table; [`textures.py`](textures.py) holds one
recipe per scheme. The dust film never greys rubber or glass, and only paint
takes its side's tint.

Roster vehicle exporters build to the physical frame of the unit type that
draws their appearance, read from the resolved catalogs by
[one helper](catalog_frames.py); a family is the appearances whose sources lie
in its folder. Nothing else supplies a frame, so an appearance no unit draws
cannot be exported, and its tests run without Blender:
`python3 -m unittest discover -s packages/scene-assets/blender -p 'catalog_frames_test.py'`.

A family's script (`roster/<family>.py`) says only what its vehicle looks like and how it is wrecked; [the shared
export run](vehicle_export.py) does the rest the same way for every family:
variants and frames, materials by role, mount rigs, crew, tiers, wreck and
export.

A disabled card has no unit type, so it has no catalog frame: its family
script states one from its references: the length, width and height.
`vehicle_export.run_disabled` builds it to that
frame (`catalog_frames.disabled_variant`), refuses a model that strays from it
(leaving out what `skip` names: dressing, guns, rotor blades), and writes the
family's receipt beside the cards' GLBs (in `assets/source/roster/disabled/`,
or the ground cards' own family folders). A card that gains a unit type
leaves the manifest, and its sources move to its family's folder, named for
its reference library: from then on it builds through `vehicle_export.run`
like any roster vehicle. Every receipt hashes each script the
export loaded and names the reference library of its source's folder (a
manifest entry's `source_family`).
The support cards' trucks stand on [one chassis](roster/truck_chassis.py)
(rails, axles, fenders and the cab shapes their references show). A disabled
infantry card is a kit in `KITS` like any roster kit; each look is its own
export, so the kit's receipt is written once all are (`--receipt`). Aircraft and rotorcraft parts (fuselages,
flying surfaces, canopies, intakes, nozzles, gear, stores, rotors, skids) and
the crash every airframe's wreck starts from have [their own
owner](aircraft_parts.py) beside the vehicle parts, and a whole jet from its
spec [its own](aircraft_jet.py). A body lofted through drawn cross-sections and
a bubble canopy drawn from its outlines live beside them
([`aircraft_loft.py`](aircraft_loft.py), [`aircraft_canopy.py`](aircraft_canopy.py)).

A jet's shell can instead be a generated skin ([`aircraft_skin.py`](aircraft_skin.py)):
an image-to-3D model's mesh of the airframe alone, made from shaded side, top
and bottom views of a model measured against the references, so it keeps the
measured proportions as one smooth, blended surface. Each is paid for once
and pinned in [`skins/<family>.json`](skins/) with its hash, request and
inputs (`assets/source/skins/<family>/`); the export refuses any other file
and its receipt names the skins it wore. Its canopy is fitted to its own
hump; everything else (gear, stores, nozzles, markings) is still built here,
seated on the skin. To make one: build the airframe's guide, a lofted model measured against
its references and kept in `guides/<family>.py` (exported with `--out`
and `--no-lint`: the shipped model is untouched, and a guide carries none of
its fittings), draw its guide views
([`skin_views.py`](skin_views.py)), generate and pin
the skin from them ([`../tools/skin-generate.mjs`](../tools/skin-generate.mjs),
paid, with `FAL_KEY`), measure it against that model
([`skin_register.py`](skin_register.py)) for the `lower` and `span` the
family's spec gives `skin`, and check it on its sheet. A rotorcraft's frame leaves
its blades out (`blade_*`): length nose to tail, width over the widest fixed
part, height to the top of the rotor head.

Every vehicle has its own wreck. A roster exporter given `--wreck` writes its
vehicle's wreck beside the live one (`<appearance>_wreck.glb`, and
`_hull`/`_turret` pieces where it has a turret to throw), burnt through
[one helper](wreckage.py) after the family models its own damage (wheels and
plates torn off, warped and dented hulls, litter beside the hull). The same
helper throws debris clear of the hull from what the vehicle carries (track
runs, wheels, hatches, doors, armour packs, stowage: `wreckage.scatter`), and
writes it as the wreck's `debris` state (`_wreck_debris.glb`): it lies past the
box the simulation keeps as cover, so the battle lets it sink away. A family
names nothing `debris_*` itself. Its tests build scenes, so they run inside
Blender: [`wreckage_test.py`](wreckage_test.py) through the `asset` command.

Subject exporters build vehicles, wrecks, props, trees and forest-floor material.
Shared part, masonry, texture and damage helpers keep frame conventions and seeded
source identity consistent across those subjects. [The batch script](build_sources.sh)
defines its supported rebuild sequence, not every catalog source. It can leave
partial output on failure or interruption and stops at the first failed exporter.
A successful source export is still followed by
[appearance baking and checking](../README.md).

## Geometry lint

[Vehicle lint](vehicle_lint.py) catches modeling mistakes before export. It runs
headless as part of the vehicle export pipeline, checking:

- **Interpenetrating parts:** mesh-to-mesh intersection between separate
  bodies (e.g., side skirts sinking into wheels, a turret cutting into a fender
  box). A body is the hull with everything fixed to it, each piece of running
  gear on it (`wheel_*`, `track_*`, `gear_*`), or each node hung off the
  vehicle's root (the turret, with all it carries); plates and fittings fixed
  to one body overlap by design and are not compared. Uses BVH overlap
  detection with a 5mm tolerance for intended contacts and mounts.
- **Floating parts:** any part more than 2cm from every other part of the same
  vehicle (detached bits hovering in the air).
- **Ground contact:** tracks or wheels must touch the ground plane in rest pose
  within 1cm. Flags vehicles that hover above or sink below.

The lint runs automatically during export; pass `--no-lint` to skip it. New issues
not in the [baseline](vehicle_lint_baseline.json) fail the export. Known issues in
the baseline pass, giving a path to fix them incrementally.

Standalone, [the roster walk](vehicle_lint_roster.py) lints every appearance a
roster script exports, whichever folders they come from: each script runs as
its export would, but its `run` reports the variants it would build instead
of building them. A script or build that fails fails the walk, and the
baseline is only rewritten after a walk with no failures:

```bash
# Lint every appearance the roster exports
bun run --cwd web asset -- blender ../packages/scene-assets/blender/vehicle_lint.py -- --roster

# Lint one appearance, by its unit id
bun run --cwd web asset -- blender ../packages/scene-assets/blender/vehicle_lint.py -- --variant=eastern_t_72_t_72b3_2016

# Rebuild the baseline from the current issues
bun run --cwd web asset -- blender ../packages/scene-assets/blender/vehicle_lint.py -- --update-baseline
```

Tests use synthetic fixtures; the geometry checks run inside Blender
([`vehicle_lint_test.py`](vehicle_lint_test.py)), while the baseline filtering
logic has non-Blender tests ([`vehicle_lint_logic_test.py`](vehicle_lint_logic_test.py)).
`asset check` runs every `*_test.py` in this folder: one that imports `bpy`
in the pinned Blender, the rest under plain Python.

The checks are extensible: add new checks to `LINT_CHECKS` in `vehicle_lint.py`.
Planned checks include flat or missing materials, texture setup, and hull side
profile versus reference.

[City authoring](city/README.md) owns kits, template descriptors, interiors and
source-side review. Procedural grass and the generic prop stand-in are generated
by the asset CLI's TypeScript owners rather than this Blender batch. Changing a
shared helper requires rebuilding the affected sources, not merely rebaking old GLBs.

[Pack acquisition](packs.py) is a regular Python tool, independent of Blender.
It downloads and verifies [pinned inputs](packs.json) into a local cache; script
reads verify those pins again. Its module usage owns acquisition arguments and
`BATTLEGAME_PACKS` cache selection. [Credits](../../../assets/README.md#third-party-sources)
own licensing. Vendored graph sources retain their [notice](vendor/procedural-buildings/README.md).

Source-only facade checks use regular Python; rendered review and exporters need
Blender. [City verification](city/README.md#from-a-set-to-a-town) distinguishes those
checks from game-rendered fit and material review. Helper imports are not evidence
that a source export or its runtime bundle is current.
