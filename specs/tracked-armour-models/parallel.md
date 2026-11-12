# Lanes, ownership, merging and baking

The coordinator runs slices 00–02, 08–16, 30 and 31, and owns every file in the
list below. The other slices run as lanes. Each lane is an Opus subagent in its
own worktree, branched from the commit named for it. Its brief is its slice
file, this page, and `slices/family-template.md` (for family lanes).

## Coordinator-only files

- **Blender modules:** the eight vehicle modules from slice 02 (`vehicle_parts`,
  `running_gear`, `hull_shape`, `armour_parts`, `roof_parts`, `hull_fittings`,
  `stowage`, `vehicle_guns`), plus `scripted_body.py`, `mesh_lods.py`, `parts.py`, `vehicle_export.py`,
  `vehicle_lint*.py`, `vehicle_lint_allowlist.json`,
  `vehicle_lint_baseline.json`, `catalog_frames.py`, `wreckage.py`,
  `textures.py` and `vehicle_crew.py`.
- **`aircraft_skin.py`.** Slice 14, if it runs, is a coordinator slice.
- **Data and catalogs:** `assets/catalog.json`, `fixtures/**`, `crates/**`,
  `web/**`, `assets/runtime/**`.
- **Spec files:** this spec's `README.md` and `parallel.md`.

## Lane files

Every lane also owns:
- its own slice files;
- `throwaway/argon/<vehicle>/` for its briefs and session results;
- `specs/tracked-armour-models/assets/{after,experiments,frames}/<its family or experiment>/`;
- the right to append to `decisions.md`.

| Lane | Branches from | Owns |
|---|---|---|
| A, Eastern tanks (17–19) | the rollout plan commit | `t80`, `t90`, `type99`, `t14`, `t15`, `type15` roster scripts (not `t72` or `armata`, which they import), and their `assets/source/roster/<folder>/`, `assets/source/skins/<family>/` and `assets/references/<family>/` |
| B, Western tanks (20–23) | the rollout plan commit | `challenger`, `challenger_3`, `abrams`, `m1e3`, `m10`, `leopard`, `leclerc`, `kf51`, and their source, skin and reference folders |
| C, IFVs and recon (25–28) | the rollout plan commit | `cv90`, `cv90120`, `bradley`, `puma`, `ajax`, `bmp`, `brm`, and their source, skin and reference folders |
| D, artillery and air defence (24, 29) | the rollout plan commit | `air_defence` (plus a new `buk.py` if it splits), `rocket_artillery`, `sp_howitzers`, and their source, skin and reference folders, including the wheeled siblings' source folders it re-exports (`pantsir`, `boxer_skyranger_30`, `nasams`, `m142_himars`, `bm_21_grad`, `9k515_tornado_s`) and `s300`'s, which it only re-exports. `boxer.py`, `truck_chassis.py` and `ural.py` are read-only. |

Earlier lanes (E1, E2, J, F-A/B/C, and the old A/B/C of the procedural plan)
are merged; their rows are in git history.

**Shared procedural code stays.** A family script that wears its body stops
calling its procedural parts, but a lane never deletes a function another
family still imports (the T-72's for the T-80, T-90 and Type 99, Armata's
for the T-14 and T-15, the CV90's for the CV90120, the Abrams' for the M1E3).
Slice 30 deletes what nothing calls any more.

**Argon spend.** Each lane keeps a running cost per session (the turn's
`usageByModel` cost) in its slice files; at most two rounds per vehicle (A6).

Use the real folder names under `assets/source/roster/`. Check them with `ls`
before the lane starts.

### When a lane needs a shared helper changed

The lane never edits the helper. It writes a **shared-part request** (what it
needs, why, and the call it wants) in its slice file and in `decisions.md`.
It then carries on with its other items and rebases once the coordinator lands
the amendment.

A family-local function that only forwards fixed arguments to a helper is
allowed, because that is the family's data. For example, `soviet_skirts` may
call `AR.side_skirt`. Copying, wrapping or monkey-patching a helper is not
allowed.

## Rules in every worktree

- **Builds.** Run `bun run build:wasm` in the worktree before any `asset
  validate`, `asset sheet` or bake (`web/src/battle/catalog/node.ts:41-44`).
  Never share build output across worktrees.
- **LFS.** Pull only what you render or export:
  `git lfs pull -I "assets/source/roster/<folder>,assets/references/<family>"`.
  Use directory patterns. `**` globs don't materialise files.
- **GPU lock.** Any GPU step (`asset sheet`, browser captures, battle-camera
  shots) runs under `lockf -k <main checkout>/throwaway/gpu.lock`.
- **Blender.** Always export through the asset CLI, which pins Blender 5.2.1
  and runs it on one thread (`web/asset.mjs:101-102,509-517`). Running Blender
  directly gives exports that aren't byte-identical.
- **Receipts.** A lane may bake in its own worktree to look, but never commits
  `assets/runtime/**`. Only full-family runs write receipts, and receipts are
  LFS binaries that can't be merged. Exactly one lane owns each receipt path.
- **Lint baseline.** Lanes never run `vehicle_lint.py -- --update-baseline`. A
  family must reach zero lint issues for its own appearances (not just zero
  new ones). Stale baseline entries are harmless until slice 30 regenerates the
  baseline.
- **Cleanup.** Remove the worktree and its build output once its branch is
  merged.

## Merging

At each merge, the coordinator compares `git diff --name-only <base>..<lane>`
against the lane's file list above. Any file outside the list is sent back to
the lane.

Lane files are disjoint, so family commits merge in arrival order, one commit
per family. After merging, the coordinator re-exports nothing; the lane's
committed GLBs are the record. The exception is the receipt freshness probe at
slice 30.

## Baking and pushing

Only the coordinator bakes, and only on the merged tree. Every push to
`staging` is a milestone, and at every milestone:

1. `bun run --cwd web asset -- bake` on the merged tree, then
   `bun run --cwd web asset -- check` green;
2. the narrowest checks the merged slices need (each slice names them);
3. `git push` to `staging`, then `bun run promote:main` (fast-forward only).

The milestones are:
- after 01;
- after 02 (no bake needed if 02 is byte-identical);
- after 05;
- after 16;
- after each lane finishes, or after any three family merges, whichever comes
  first;
- after 31.

`bun run check` reads the baked runtime against the frames
(`web/tests/sceneAssets/unitFit.test.ts:91-110`). Never push frame changes
without their re-exports and the bake.
