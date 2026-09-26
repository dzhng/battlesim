# scene-assets

The one owner of appearance bundles: schema, validation, baking and loading. The workbench and the battle load through the same `AppearanceLibrary` (`src/loader.ts`). The CLI (`web/asset.mjs`) is only file IO around this package.

The contract is ported from `~/dev/game`'s soldier-assets (`ART_INPUT_CONTRACT.md`); the encoding is ours (`specs/battle-look/decisions.md`, "Bundle encoding").

## Principles

- **Atomic.** A bundle is complete or refused. The loader installs a whole catalog generation at once and keeps the old one when anything fails.
- **Content-addressed.** A bundle is one binary file named by the sha256 of its bytes, under `assets/runtime/<hash>/bundle.bin`. The bake is deterministic, so a hash changes only when content does. The loader checks every hash.
- **Clips once per skeleton.** Skinned bodies carry joints, binds and meshes. Animation lives in one `SkeletonClips` bundle per skeleton, shared by every body on that rig.
- **Engine space is decided at bake.** Z up, +X forward, +Y left, metres, origin on the ground. The bake applies glTF's Y-up conversion plus the catalog's `basis_yaw_deg` (90 for the Quaternius rig) as one transform above the roots. The loader never converts.
- **The simulation is the authority on fit.** Soldier height, eye and muzzle, hull extents and the tank muzzle are read from the fixture's `physics` block. How far art may sit from them is a catalog tolerance, not a rule: widen a tolerance, per appearance if needed, rather than weakening a check.
- **Every source is provenance-checked.** Its content hash must be a `third_party` entry of `specs/battle-look/assets/reuse-manifest.json` with an allow-listed licence. An LFS pointer is hashed by its oid, so checks need no pull.

## Source conventions (what the validator expects of a GLB)

- Four tiers in one GLB, named by a `_LOD0`..`_LOD3` suffix on mesh objects, finest first. An unsuffixed mesh is in every tier.
- **Skinned** (infantry): one skin, at most four weights. Unweighted `_leaf` joints are dropped unless the skeleton has them. Kit parented to a bone is skinned rigidly to it at bake. Sockets are empties under a joint; infantry needs `eye` and `muzzle`. The skeleton needs the six clip roles (`INFANTRY_CLIPS`), each with an explicit loop flag, plus a standing-aim reference pose where the muzzle is measured.
- **Articulated** (vehicles): one root empty. Every empty is a moving node; mesh objects fold into their nearest empty. Node frames are engine-aligned, so a turret yaws about its local +Z. The required names per unit are in `validate.ts`. Tracks carry `track_length_m` and `link_pitch_m` custom properties. The turret must yaw about the hull origin, as the simulation's muzzle model does; this is checked as an arc.
- **How vehicles move** is `articulation.ts`: the pose inputs (`Articulation`: turret and gun, HMG, each side's travel, deploy progress) mapped onto named nodes. Wheels roll about their local +Y; guns pitch about their local +Y. Deploying parts carry their own motion as custom properties (`deploy_start`, `deploy_end`, `deploy_move_{x,y,z}`, `deploy_turn_{x,y,z}`), and a supply truck's pads must reach the ground at deploy 1. An articulated bundle's `bounds` cover every pose this reaches; fit is still measured at rest.
- **Static** (buildings and scenery): one GLB per state. Buildings need `intact` and `ruin`.
- **Scenery** (`unit: "scenery"`): every prop, tree, hedgerow and grass kind, named by the entry's `scenery`. A scenery kind is one row of `SCENERY_KINDS` (`src/scenery.ts`), the one extension point: the states its art must carry, and its footprint, meaning what the simulation knows of it. The footprint is a prop kind (box and blocking class), a forest tree (trunk and canopy) or nothing. To add a kind, add a row. The validator, the bake, the loader and the workbench's overlay all read it from there.

## Where things are

`schema.ts` holds the types and constants, `validate.ts` the rules and finding codes, and `codec.ts` the binary layout. `loose.ts` judges one GLB on its own, for `asset validate` and the model workbench's drop zone. Every validation also returns a `preview`, the bundle as built even when it has errors; the workbench installs previews through the same loader (`previewRuntime` plus `memoryFetch`), and the bake never writes them.
