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
Clips and meshes must retain the shared skeleton contract: regenerating one does
not regenerate the other. Shared rig, weapon and mesh-tier helpers concentrate
that policy rather than defining extra standalone asset commands.

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
