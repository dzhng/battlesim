# 30: lineup and sweep (coordinator)

## Contract

The 37 models read as one cohesive lineup beside their references. The end
state matches the single-owner table in the README. Every receipt is fresh, and
the lint baseline holds no tracked appearance.

## Steps

1. **Merge.** Merge every lane following the rules in `../parallel.md`.
2. **Receipt freshness probe** (in `throwaway/`). Find every tracked receipt
   whose `source_sha256` no longer matches HEAD. That happens when a shared
   amendment landed after the family merged. Re-export each of those families
   in full.
3. **End-state greps,** which must all come back empty:
   - `skirt`, `fender` or `side_module` drawn with a raw `box(`/`prism(` in a
     tracked script;
   - `thermal_sleeve_` drawn with `cyl(`;
   - `head_out` in any tracked script;
   - `def skirt_top`, `def skirt_foot` or `def follow_tiles`;
   - `lid_open` in any tracked roster script. The test tank's `driver_lid_open`
     (`tank.py:559`) is test art and outside this pass (C17).
4. **Stale tolerances (R7).** Drop each `tolerances.hull_extent_m` in
   `assets/catalog.json` that only a hull-drawn barrel needed (the SP
   howitzers now ride the `gun` rig), keeping any the body itself still
   needs; `asset validate --type` decides.
   Drop each wreck's `tolerances.footprint_m` of 1.0 or less too (R10:
   the wreck kind's `rubble_m` floor covers it); `asset validate` decides.
   And `mesh_lods` (coordinator-only): tiers 2–3 keep stray degenerate
   vertices past the hull (lane D: the M270's `hull_armor_paint_LOD3` at
   x -4.03), harmless to validate but they inflate bounds; drop them.
5. **Swept turret fronts (R12).** Measure every turret's plan outline (its
   foremost x across |y|, mantlet and gun excluded). From the mantlet's edge
   outward the front must run back: never level, never coming forward. Fix
   each that fails in its `build.py`.
6. **Lint baseline.** Run `vehicle_lint.py -- --update-baseline`, then check
   that no tracked appearance id is left in it.
7. **Lineup cohesion sheet.** Write `visualizations/lineup.html`: all 37 at
   `battle_zoom` and `cinematic`, beside their references, plus a montage PNG
   in `assets/after/`.
8. **`compare-screenshots`** of the lineup against `assets/before/`.
9. **Last check: an unprimed `screenshot-critique`** of the montage, for
   cohesion. That means consistent scale, finish, edge treatment and level of
   detail across nations.
   - Fix a cohesion outlier in its family script. Up to three rounds.
10. **Below the bar.** Collect every model still below the bar, from each
   slice's scores, into the README for the user.

## Delegated

The montage layout.
