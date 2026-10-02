# C10: third-party sources

**Depends on:** G0. **Kind:** slice.

## Question
Is every vendored source recorded beside it, with its URL and a licence that allows shipping?

## Contract it unlocks
- A third-party source is recorded beside the asset: its source URL and its licence, in the readme of the folder that holds it. There is no manifest, no hash allow-list and no validator.
- The three `.blend` files go in Git LFS under `packages/scene-assets/blender/vendor/procedural-buildings/`, as source of truth (Q-F′), with their MIT notice kept beside them.
- ambientCG sets at ≤1K are read from the local pack cache and never committed. `packages/scene-assets/blender/packs.json` pins each by hash, with its URL and licence, as it does the soldier packs.
- The two unknown-source atlases are never committed (Q-E).

## API seam
The readme beside each vendored source; `packages/scene-assets/blender/packs.json`, read only by `packs.py`.

## What the human can run or see
The readme entries, and `packs.py fetch` verifying the cached packs.

## Verification
- `packs.py` stops on a cached pack whose hash differs from its pin.
- Review at the slice's commit: every third-party file it adds has its URL and licence beside it, and neither atlas is in the tree.
- Art records and map-data records (`SOURCES.json`) stay separate.

## Delegated to the implementer
Pack cache layout. Anything else you have to decide is a spec gap: record it in `../choices.md` under this slice.

## Must stay green
`asset check`; the existing pack pins.

## Feedback that would change this slice
An unacceptable licence or untraceable source changes the source choice before its art is baked.

## Outcome

The three `.blend` files are in Git LFS under `packages/scene-assets/blender/vendor/procedural-buildings/`, copied unchanged from the upstream repository at the commit its readme names, with the MIT notice and a readme beside them. Neither photo atlas is in the tree.

**Open:** the ambientCG pins. A set is pinned in `packs.json` by the pass that first reads it (C12), since only that pass knows which sets survive the bake; `packs.py` learns ambientCG's direct downloads then.
