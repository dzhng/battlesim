# 00: lint truth (coordinator)

## Contract

The geometry lint does what its docstrings say. When this slice is done:

- the lint compares parts for overlap on real vehicles;
- it walks every appearance a script exports;
- it fails when a build fails;
- `--update-baseline` refuses to write a partial baseline;
- the five Blender-side Python tests run in `asset check`.

Exports are also proven byte-repeatable, and their wall time is known.

## Why first

Exports fail on any new lint issue (`vehicle_export.py:56-67`). If the lint
starts telling the truth after the baseline is recorded, every later diff picks
up old issues. Decision S1.

## Seam

All of this is in `packages/scene-assets/blender/`.

- **Grouping.** `vehicle_lint._part_group` (`vehicle_lint.py:243-259`) walks
  ancestors from the root down and returns the first one that is not `hull`.
  That is always the vehicle root, the node carrying `unit_id`
  (`vehicle_export.py:127-128`; the turret's carrier is also the root,
  `:105`). So every mesh lands in one group and every pair is skipped
  (`:400-401`).
  - Fix: skip the root. Group by the first articulated or hull-child node under
    it.
  - Granularity follows decision S1: keep each hull child its own group, unless
    the T-72B3 alone shows more than 30 overlap issues that are mostly
    plate-on-plate by design. In that case, group at `hull`, `turret`,
    `wheel_*` and `track_*`, and record the call.
- **Roster walk.** `--roster` and `--update-baseline` (`vehicle_lint.py:905-1063`)
  derive one family per script from its first `run(` (`_get_family_name_from_script`).
  - Walk every appearance the script exports instead: its `appearances=` list,
    or its catalog `family_variants` (`catalog_frames.py:95-102`).
  - Move the roster walk into a new `vehicle_lint_roster.py`.
    `vehicle_lint.py` is 42,691 B, over the 32,000 B ratchet, so it must
    shrink, not grow.
- **Failure handling.**
  - Exit non-zero when any import, build or skip fails, in both modes.
  - `--update-baseline` rebuilds the file from `{}` (`:944-946`). Make it refuse
    to write while any family failed.
- **Tests in `web/asset.mjs` `check`** (`:318`). Run the five tests and report
  them separately from the bake results:
  - `catalog_frames_test.py`, `vehicle_lint_logic_test.py` and
    `vehicle_export_test.py` with `python3 -m unittest`;
  - `vehicle_lint_test.py` and `wreckage_test.py` through the pinned Blender
    (blender `README.md:72-73,133,174-176`).

## Steps

1. **Determinism probe.**
   - Export the T-72B3 twice, live and with `--wreck`:
     `bun run --cwd web asset -- blender ../packages/scene-assets/blender/roster/t72.py -- --variant=eastern_t_72_t_72b3_2016 --out=<throwaway path>`.
   - The two exports must have identical SHA-256 hashes. Record the wall time in
     this file.
   - If the hashes differ, stop. Fix the trap first: see the make-3d-models
     Blender references and the `noise_vector` note in
     `specs/done/unit-models/README.md`. Every identity proof later depends on
     this.
2. **Lint the T-72B3 alone** before and after the grouping fix:
   `… vehicle_lint.py -- --variant=eastern_t_72_t_72b3_2016`. Count the issues
   and apply rule S1.
3. Fix the roster walk and the failure exits.
4. Add one case to `vehicle_lint_test.py`: a hull plate and a turret plate that
   overlap under a real root (`unit_id`) are flagged. It pins the behaviour the
   synthetic test (`:99-110`) misses, because that test has no root.
5. Wire the tests into `asset check`. Run each once by hand first. A test that
   was already broken is fixed in its own commit.
6. Run `--roster`. Confirm the summary names Buk-M3, MARS II, PzH 2000, 2S19M2
   and PLZ-05.

Don't run `--update-baseline` yet. Slice 01 records the baseline after the
re-export.

## What the human can see

A lint summary listing every appearance, issue counts by check, and the
determinism and timing note.

## Verification

- The new `vehicle_lint_test.py` case fails before the fix and passes after.
- `--roster` exits non-zero while any family fails to build.
- `asset check` runs and reports all five tests.
- There's no screenshot gate, because nothing visual changes.

## Must stay green

- Every family still builds under `--roster`.
- The ratchet: `vehicle_lint.py` shrinks.
- `bun run check` is untouched.

## Delegated

- module and function names in `vehicle_lint_roster.py`;
- how appearances are enumerated (the `appearances=` list or catalog
  `family_variants`).

## Feedback that would change it

If the user doesn't want overlap issues lit up across the whole roster: keep
the fix, and limit what it surfaces to tracked appearances through the
baseline.
