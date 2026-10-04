# 08 Art: shared core, then China, New York, Paris

Four sub-slices, each closed on its own: **08a** shared core (bench, bins, bike rack, playground, and the garden pieces); **08b** China; **08c** New York; **08d** Paris.

**Seam:** Blender scripts in `packages/scene-assets/blender/`. Follow `street.py` exactly: one script, one decorated function per kind, each authored to its catalog box (origin at the box's centre on the ground, +X along the first half extent), materials from `parts.py`/`textures.py` recipes, `box_uv` UVs, fixed seeds so the GLB is byte-deterministic, and one line per kind in `build_sources.sh`. New pieces live in a new `courts.py`; a regional variant of a shared piece (a Paris bench) is a second function writing a second source with the same box, registered as scenery kinds (`packages/scene-assets/src/scenery.ts`), appearances tagged `regional_family` where regional, baked with the `asset` tooling; provenance in `assets/README.md`. Every piece within slice 00's byte cap and inside its catalog box (boxes do not move: they are physics).

**Pieces per sub-slice:** 08a the slice 05 shared kinds and the slice 06 garden kinds; 08b–d each region's signature kinds plus its variants of the shared pieces that should read regional (benches, bins, bike rack; the playground can stay shared). Reuse `street.py`'s recipes before adding new ones, and bake textures at the size the 1 MiB cap allows.

**Verify:** `asset check`; the budget test; shots per region at the town stations and a close court station; screenshot-critique; compare-screenshots against slice 05's stand-in shots and any reference photos added to `../assets/`.

**Review checkpoint (non-blocking), one per region:** is the region recognisable from 120 m?

**Delegated:** modelling, palettes, LOD tiers; New York's dumpster yard dressing.
