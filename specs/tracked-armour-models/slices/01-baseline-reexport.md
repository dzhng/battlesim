# 01: baseline re-export (coordinator)

## Contract

Every tracked script's committed GLBs, wrecks and receipts match today's
sources, and the lint baseline holds the roster's real issues as they stand.
After this slice, every diff shows only this pass's work.

The standard shot list exists, and the "before" record is complete.

## Seam

No new code. This slice uses:
- full-family runs (only those write receipts: `vehicle_export.py:218`);
- then `-- --wreck` for each script;
- then `vehicle_lint.py -- --update-baseline`.

## Steps

1. **Re-export all 25 tracked scripts, live then `--wreck`**, in the
   background, with progress written to `throwaway/`.
   - `sp_howitzers`, `air_defence` and `rocket_artillery` export all of their
     cards, which includes the wheeled siblings.
   - Also re-export `s300.py`, because it imports `rocket_artillery`.
2. **Lint:** `vehicle_lint.py -- --roster`.
   - Fix trivial issues on sight. A trivial issue is a part floating less than
     2 cm, or an obvious misplacement, and each fix gets its own commit.
   - Then run `--update-baseline` once, so every other issue is recorded.
   - List the baselined issues for each family in this file, sorted by the lane
     that owns the family. Family slices must clear their own.
3. **Before record.** `assets/before/` already holds the planning lineup.
   - Add fresh closeups of the 37 re-exported models in
     `assets/before/reexport/`.
   - Run `compare-screenshots` of them against the planning closeups. Drift from
     stale helpers is expected. Name it in `decisions.md`; don't fix it.
4. **Triangle table.** Write the `triangles_by_tier` of the 37 models from their
   receipts into this file, next to their caps.
5. **`assets/shots.txt`:** the `--shot name:ex,ey,ez:tx,ty,tz[:lens]` lines
   every later slice uses. There are two shots.
   - `battle_zoom`: the closest battle-camera distance and pitch, read from the
     presentation camera config. Record the file and line you read it from.
   - `cinematic`: a low three-quarter front, at about 1.5 m eye height, 8–12 m
     out, with a long lens.
   - Plus the existing `fq` and `side` shots.
6. **Planning scratch: already done.** It moved to `throwaway/tracked-lineup/`
   when the spec was written (decision S15). Confirm `git status` shows no
   `scratch/`.
7. **Push milestone:** bake on this tree, then `asset check`, then push, then
   promote (see `../parallel.md`).

## What the human can see

- the receipt diff;
- the lint baseline diff, grouped by family;
- `assets/before/reexport/`;
- the triangle table.

## Verification

- Every export passes against the baseline it just wrote.
- `asset check` is green after the bake.
- `bun run check` passes on the narrow tests it touches: `web test` unitFit,
  because the frames are unchanged.

## Must stay green

- frames;
- digests (nothing simulated changes);
- the ratchet.

## Delegated

- the batch order;
- what counts as a trivial lint fix, within the limit in step 2.

## Feedback that would change it

If export wall time per script is above about 5 minutes, schedule batches and
record the timings here.
