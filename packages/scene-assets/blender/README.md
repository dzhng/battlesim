# Appearance source authoring

These scripts make committed source art; they are not part of game startup or
ordinary bundle baking. [The asset CLI](../../../web/asset.mjs) launches headless
Blender and checks the pinned version. Set `BLENDER` to another installation path
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
A roster kit several factions field is built once per army
([roster equipment](roster/infantry_equipment.py) `ARMIES`, named as its reference
variants): each army's uniform print ([`textures.py`](textures.py) `UNIFORMS`),
helmet, carrier and rifle on the same rig, the appearance's faction looks.
Clips and meshes must retain the shared skeleton contract: regenerating one does
not regenerate the other. Shared rig, weapon and mesh-tier helpers concentrate
that policy rather than defining extra standalone asset commands.

Visible vehicle crew ([crew module](vehicle_crew.py)) reuse their faction's
exported soldier art in fixed operating poses, from the soldier's tier 1 down
and without the weapon. They belong to the vehicle's appearance and mount
hierarchy, rather than adding simulation soldiers. Re-export their vehicles
after changing the infantry source; the crew exporter preserves that source's
exact textures and material masks on the crew's own materials only.

Reusable vehicle detail (wheels, running gear, hatches, sights, lights, tow
points, stowage, grilles, exhausts) has [one owner](vehicle_parts.py); a family
script places those parts rather than redefining them.

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

A rebuilt family's script (`roster/abrams.py`, `stryker.py`, `humvee.py`, …)
says only what its vehicle looks like and how it is wrecked; [the shared
export run](vehicle_export.py) does the rest the same way for every family:
variants and frames, materials by role, mount rigs, crew, tiers, wreck and
export.

Every vehicle has its own wreck. A roster exporter given `--wreck` writes its
vehicle's wreck beside the live one (`<appearance>_wreck.glb`, and
`_hull`/`_turret` pieces where it has a turret to throw), burnt through
[one helper](wreckage.py). A rebuilt family models its damage first (wheels
and plates torn off, warped and dented hulls, debris); a family not yet
rebuilt has only the interim burn.

Subject exporters build vehicles, wrecks, props, trees and forest-floor material.
Shared part, masonry, texture and damage helpers keep frame conventions and seeded
source identity consistent across those subjects. [The batch script](build_sources.sh)
defines its supported rebuild sequence, not every catalog source. It can leave
partial output on failure or interruption and stops at the first failed exporter.
A successful source export is still followed by
[appearance baking and checking](../README.md).

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
