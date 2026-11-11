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
   - `lid_open` anywhere.
4. **Lint baseline.** Run `vehicle_lint.py -- --update-baseline`, then check
   that no tracked appearance id is left in it.
5. **Lineup cohesion sheet.** Write `visualizations/lineup.html`: all 37 at
   `battle_zoom` and `cinematic`, beside their references, plus a montage PNG
   in `assets/after/`.
6. **`compare-screenshots`** of the lineup against `assets/before/`.
7. **Last check: an unprimed `screenshot-critique`** of the montage, for
   cohesion. That means consistent scale, finish, edge treatment and level of
   detail across nations.
   - Fix a cohesion outlier in its family script. Up to three rounds.
8. **Below the bar.** Collect every model still below the bar, from each
   slice's scores, into the README for the user.

## Delegated

The montage layout.
