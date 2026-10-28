# 06 Starting values (changes battles)

**Unlocks:** the decided starting values ship. This slice changes **data only**. If any code needs to change, slice 04 was wrong: stop and reslice.

Run `tweak-mechanics` first, and picture each case as a war film.

## Data

- `sensors.moving_concealment`: 1.5.
- Squads: scouts and elite scouts 0.8, snipers 0.5, everyone else 1 (README table).
- Spotting: snipers 2 (added), the five `*_mbt` profiles 0.8, recon unchanged.
- **Hulls by box volume**, every hull including aircraft and the test and capture hulls: ≤ 35 m³ → 1.8, 35–65 m³ → 2.4, > 65 m³ → 2.8. A one-off script in `throwaway/` computes `8 × x × y × z` from each resolved hull's `half_extents_m` and writes literal values into the authored files. Commit only the data.

## Verification

- Rerun the report before and after, and judge the change against the README's starting-values table.
- **Re-record:**
  - `BLESS_PARITY` for the simulation records;
  - the menu reel log (`BLESS_MENU_REEL`);
  - the browser capture runs in `fixtures/parity/capture/` (run, counts, poses, publication);
  - any presentation tests that replay those battles.
- Inspect where each re-recorded battle first diverges, and confirm replay equality.
- Judge the reel footage yourself (`compare-screenshots` before and after, then an unprimed `screenshot-critique`). If a followed unit now dies early, change the reel's own units or scenario, never the mechanic.
- The slice 04 system test and the `effective` unit test stay green **without edits**. That proves they test mechanics, not configuration.

## Delegated to you

Tuning the capture units for the reel, and judging the reel yourself.
