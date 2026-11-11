# Lanes, ownership, merging and baking

The coordinator runs slices 00–02, 08–16, 30 and 31, and owns every file in the
list below. The other slices run as lanes. Each lane is an Opus subagent in its
own worktree, branched from the commit named for it. Its brief is its slice
file, this page, and `slices/family-template.md` (for family lanes).

## Coordinator-only files

- **Blender modules:** the eight vehicle modules from slice 02 (`vehicle_parts`,
  `running_gear`, `hull_shape`, `armour_parts`, `roof_parts`, `hull_fittings`,
  `stowage`, `vehicle_guns`), plus `parts.py`, `vehicle_export.py`,
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
- `specs/tracked-armour-models/assets/{after,experiments,frames}/<its family or experiment>/`;
- the right to append to `decisions.md`.

| Lane | Branches from | Owns |
|---|---|---|
| E1, cast turret (03) | 02 | `throwaway/`, `assets/experiments/03/`; the generated skin and its pin (`packages/scene-assets/blender/skins/`, `assets/source/skins/` if that is where pins put the mesh) |
| E2, seams (04) | 02 | `throwaway/`, `assets/experiments/04/` |
| J, jets (05, 06) | 02 | `textures_air.py`, `aircraft_*.py` except `aircraft_skin.py`, every fixed-wing roster script, `guides/`, the jet source folders under `assets/source/roster/`, `assets/experiments/05/` and `06/` |
| F-A / F-B / F-C, frame survey (07) | 02 | `assets/references/<family>/**` for its lane's families, `assets/frames/<family>.*`, `throwaway/`. F-A also owns the vehicle mode added to `.agents/skills/make-3d-models/scripts/drawing_overlay.py` |
| A (17–20) | 16 | `t72`, `t80`, `t90`, `type99`, `armata`, `t14`, `t15`, `type15`, `challenger`, `challenger_3`, and their `assets/source/roster/<folder>/` and `assets/references/<family>/` |
| B (21–24) | 16 | `abrams`, `m1e3`, `m10`, `leopard`, `leclerc`, `kf51`, `air_defence` (plus the new `buk.py` and any `abrams_*` concept split), `rocket_artillery`, and their source and reference folders. That includes the sibling folders `gepard`, `buk`, `pantsir`, `boxer_skyranger_30`, `nasams`, `m270_mlrs`, `mars_ii`, `m142_himars`, `bm_21_grad` and `9k515_tornado_s`. `s300` may only be re-exported to prove it unchanged. `boxer.py`, `truck_chassis.py` and `ural.py` are read-only. |
| C (25–29) | 16 | `cv90`, `cv90120`, `bradley`, `puma`, `ajax`, `bmp`, `brm`, `sp_howitzers`, and their source and reference folders, including `pzh_2000`, `2s19_msta_s` and `plz_05` |

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
