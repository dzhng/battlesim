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

## Result (2026-11-11)

**Belt against its own wheels: artefacts, now not compared.** Closeups of
the T-72B3's running gear show a closed belt wrapping every wheel on its
side, and nothing poking through. A nearest-face inside test finds no road
wheel, idler or return roller inside the belt. Only the sprocket's teeth sit
36 mm into the 80 mm belt, which is how they drive it. The 400 mm depths
were the ray-parity test misreading the loop's hollow. The overlap check no
longer compares `track_L`/`track_R` with the `wheel_*` on its own side
(`vehicle_lint_core.compared`, one logic-test case). The T-72B3 goes from
75 overlaps to 43, and the roster from 4,990 to 3,348.

Categorising the issues also found a grouping bug from slice 00: any hull
child starting `track_` was its own body, so the Leopard's and BMP's
`track_guard` fenders were compared with the hull they are bolted to.
Running gear is now exactly `track_L`, `track_R`, `wheel_*` and `gear_*`
(`vehicle_lint_core.running_gear`), which removes 30 false issues. Both are
in `decisions.md`.

**Re-export.** All 25 tracked scripts and `s300`, live and `--wreck`, through
the asset CLI with the lint on, six at a time: 52 runs, all green against
the new baseline. Each run took a median of 33 s and at most 110 s
(`rocket_artillery --wreck`); the 52 took 6.5 minutes of wall time at load
average 8–10. The live runs were repeated after the grouping fix so their
receipts hash the final lint sources, and their GLBs came out
byte-identical. The T-72B3 now hashes `c04345c4…`, as slice 00's
determinism probe predicted.

**Held back: `air_defence.py`.** Its re-exported Buk-M3, Gepard 1A2 and
Pantsir-SM fail the bake's fit check. Their frames grew on 2026-10-15, after
the art was last exported, so the hulls now meet the frames and the tow
hooks hang 12.7 cm past them, against a 10 cm tolerance. Seating the hooks
is family geometry (lane B's slice 24, and the wheeled pass for the
Pantsir), so the script's six appearances keep their committed GLBs, wrecks
and receipt. See `decisions.md`. The other 24 tracked scripts and `s300`
are committed fresh: 35 of the 37 models.

**Trivial fix (one commit).** The SP howitzers' turret numbers hovered
2.8–3.2 cm off the turret's leaning flank. They now sit on it, tilted to
match. Nothing else qualified; see `decisions.md` for what was left and why.

**Baseline.** One `--update-baseline` after the fixes. It ran twice, because
the grouping bug was found after the first run. The second file is
committed.

| | Appearances | Overlap | Floating | Total |
|---|---:|---:|---:|---:|
| Before slice 00 | 25 | 0 | 63 | 63 |
| Now, whole roster | 116 | 3,114 | 65 | 3,179 |
| Now, the 37 tracked | 37 | 1,194 | 10 | 1,204 |

### What the 1,204 tracked issues are

Each issue is classed by the bodies it joins (the articulated grouping),
then sampled. The deepest issue of each kind in up to five families was
rendered in context and alone (`throwaway/reexport/issues/`).

| Kind | Issues | Share | Median / max depth | Appearances | On screen | To clear it |
|---|---:|---:|---|---:|---|---|
| Turret seated into hull: the turret's shell or ring guard sunk into the roof | 46 | 4 % | 48 / 137 mm | 27 | Hidden under the turret | Seat the ring on the roof: small, per family |
| Running gear seated into hull: inner tyres, hubs and sprocket/idler bodies in the lower hull's sides; the belt's upper run, return rollers and sprocket teeth inside the sponson or under the fender | 698 | 58 % | 50 / 303 mm | 37 | Hidden behind skirts, sponsons and the wheels themselves | Narrow the lower hull or move the gear out, and lift the sponson floor over the belt: real geometry work on every family |
| Fittings into armour: hull fittings under the turret's overhang (deck bolts, grilles, driver's and roof hatches) | 113 | 9 % | 25 / 438 mm | 14 | Hidden; the turret could not traverse over them | Lift the turret's underside or lower the fittings |
| Fittings into armour: turret fittings into hull fittings (ERA modules, net screens and bustle racks into fender boxes and hatches) | 93 | 8 % | 19 / 126 mm | 9 | Visible close up: the T-72 family's ERA bricks sit 10–12 cm deep in the fender-box lids | Move the boxes or the modules |
| Real misplacements: skirts, bins, lights and plates cutting into the belt or wheels (T-15 alone has 108: skirts through the road wheels' bolts and the belt) | 144 | 12 % | 34 / 365 mm | 11 | Partly; the blender README says skirts stand clear of the belt and wheels | Fix in the family; slice 09's side-panel helper |
| Real misplacements: wheels overlapping wheels on one side (an idler or sprocket into the last road wheel, a return roller into a road wheel) | 100 | 8 % | 25 / 176 mm | 8 | Visible in side views (Buk, 2S19M2, ZTZ-99A, BMP-2M) | Re-space the running gear |
| Real misplacements: floats clear (the Buk's and Gepard's unbraced side skirts, the Buk's windscreen shutter) | 5 | 0 % | 72 mm | 2 | Visible | Hang the skirts from a fender |
| Other: a part contained in another read as floating (`mg_barrel` in its receiver, the Gepard's radar sleeve) | 5 | 0 % | 25 mm | 4 | Not a defect | An allowlist pattern, as for the other barrels |

So about seven in ten issues (857: seated running gear, the turret seat,
and hull fittings under the overhang) are modelling shortcuts that never
show on screen. About three in ten (342) are real or visible misplacements,
and 5 are lint artefacts.
Every tracked appearance carries running-gear seating, from 2 issues (the
Leopards, Leclerc, T-80BVM) to 52 (BMP-2M), 494 of them in lane C. A "zero
issues" rule therefore asks every family to rework its hull sides and
running gear, beyond its lineup notes.

### Baselined issues by family and lane

T = turret seated, G = running gear seated, F = fittings into armour,
M = real misplacements, O = other. Siblings are re-exported only; their
issues belong to later passes. The full list per appearance is the
baseline file itself; a family sees its own with
`vehicle_lint.py -- --variant=<id>`.


**Lane A**

| Script | Appearance | Overlap | Floating | T | G | F | M | O |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| t72 | T-72B3 | 43 | 0 | 6 | 8 | 27 | 2 | 0 |
| t80 | T-80BVM | 20 | 0 | 4 | 2 | 14 | 0 | 0 |
| t90 | T-90M | 36 | 0 | 4 | 8 | 22 | 2 | 0 |
| type99 | ZTZ-99A | 28 | 0 | 4 | 8 | 4 | 12 | 0 |
| t14 | T-14 Armata | 9 | 0 | 1 | 8 | 0 | 0 | 0 |
| t15 | T-15 Armata | 117 | 0 | 0 | 8 | 1 | 108 | 0 |
| type15 | ZTQ-15 | 13 | 0 | 1 | 12 | 0 | 0 | 0 |
| challenger | Challenger 2 TES | 19 | 0 | 1 | 12 | 6 | 0 | 0 |
| challenger_3 | Challenger 3 | 20 | 0 | 1 | 16 | 3 | 0 | 0 |
| **lane A tracked** | | **305** | **0** | **22** | **82** | **77** | **124** | **0** |

**Lane B**

| Script | Appearance | Overlap | Floating | T | G | F | M | O |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| abrams | M1A2 SEP v2 | 10 | 0 | 2 | 4 | 4 | 0 | 0 |
| abrams | M1A2 SEP v2 Trophy | 10 | 0 | 2 | 4 | 4 | 0 | 0 |
| abrams | M1A2 SEP v3 Trophy | 23 | 0 | 2 | 4 | 17 | 0 | 0 |
| m1e3 | M1E3 Abrams | 9 | 0 | 2 | 4 | 3 | 0 | 0 |
| m10 | M10 Booker | 25 | 0 | 1 | 22 | 2 | 0 | 0 |
| leopard | Leopard 2A6 | 13 | 0 | 1 | 2 | 10 | 0 | 0 |
| leopard | Leopard 2A7V | 13 | 0 | 1 | 2 | 10 | 0 | 0 |
| leopard | Leopard 2A8 | 13 | 0 | 1 | 2 | 10 | 0 | 0 |
| leclerc | Leclerc XLR | 21 | 0 | 1 | 2 | 18 | 0 | 0 |
| kf51 | KF51 Panther | 20 | 0 | 1 | 4 | 15 | 0 | 0 |
| air_defence | Buk-M3 | 30 | 3 | 0 | 16 | 0 | 17 | 0 |
| air_defence | Pantsir-SM (sibling) | 48 | 0 | – | – | – | – | – |
| air_defence | Skyranger 30 (sibling) | 16 | 0 | – | – | – | – | – |
| air_defence | Gepard 1A2 | 40 | 4 | 0 | 16 | 0 | 26 | 2 |
| air_defence | NASAMS 3 (sibling) | 5 | 7 | – | – | – | – | – |
| air_defence | NASAMS 3 (sibling) | 5 | 7 | – | – | – | – | – |
| rocket_artillery | Tornado-S (sibling) | 81 | 2 | – | – | – | – | – |
| rocket_artillery | BM-21 Grad (sibling) | 18 | 3 | – | – | – | – | – |
| rocket_artillery | MARS II | 22 | 0 | 0 | 20 | 0 | 2 | 0 |
| rocket_artillery | HIMARS GMLRS (sibling) | 70 | 1 | – | – | – | – | – |
| rocket_artillery | HIMARS PrSM (sibling) | 70 | 1 | – | – | – | – | – |
| rocket_artillery | M270A2 MLRS | 22 | 0 | 0 | 20 | 0 | 2 | 0 |
| s300 | S-300PMU2 Favorit (sibling) | 0 | 3 | – | – | – | – | – |
| **lane B tracked** | | **271** | **7** | **14** | **122** | **93** | **47** | **2** |

**Lane C**

| Script | Appearance | Overlap | Floating | T | G | F | M | O |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| cv90 | CV9040C | 45 | 0 | 1 | 42 | 0 | 2 | 0 |
| cv90 | CV90 Mk IV | 45 | 0 | 1 | 42 | 0 | 2 | 0 |
| cv90120 | CV90120 | 65 | 0 | 1 | 42 | 22 | 0 | 0 |
| bradley | M2A4 Bradley | 45 | 0 | 1 | 44 | 0 | 0 | 0 |
| bradley | M3A3 Bradley | 45 | 0 | 1 | 44 | 0 | 0 | 0 |
| puma | Puma Level C | 44 | 0 | 0 | 44 | 0 | 0 | 0 |
| ajax | Ajax | 47 | 0 | 1 | 46 | 0 | 0 | 0 |
| bmp | BMP-2M Berezhok | 76 | 0 | 2 | 52 | 14 | 8 | 0 |
| bmp | BMP-3 | 29 | 0 | 1 | 26 | 0 | 2 | 0 |
| brm | BRM-3K | 65 | 0 | 1 | 50 | 0 | 14 | 0 |
| sp_howitzers | 2S19M2 Msta-S | 50 | 1 | 0 | 18 | 0 | 32 | 1 |
| sp_howitzers | PLZ-05 | 20 | 1 | 0 | 14 | 0 | 6 | 1 |
| sp_howitzers | PzH 2000 | 22 | 1 | 0 | 16 | 0 | 6 | 1 |
| sp_howitzers | M109A7 Paladin | 20 | 0 | 0 | 14 | 0 | 6 | 0 |
| **lane C tracked** | | **618** | **3** | **10** | **494** | **36** | **78** | **3** |

### Before record and drift

`assets/before/reexport/` holds the fq and side closeups, shaded and matcap,
of the 37 models as committed by this slice; the Buk and Gepard are their
held-back art. compare-screenshots against `assets/before/closeups/` finds
117 of the 148 pixel-identical. The movers, named in `decisions.md`:

| Model | Distance (fq shaded) | What moved |
|---|---:|---|
| MARS II, M270A2 | 0.009, 0.007 | The windscreen shutters lie over the windscreens instead of standing 26 cm proud |
| M109A7, PzH 2000, 2S19M2, PLZ-05 | ≤ 0.0003 | This slice's turret-number fix |
| Ajax | 0.0001 | The crew meshes lost one polygon (infantry source) |

The S-300 sibling is pixel-identical; its bytes differ only by one part's
last rotation bit and one UV accessor. Had it been committed, the
`air_defence` re-export would have moved the Gepard (0.056) and the Buk
(0.039): both rebuilt to their current, longer frames.

### Triangles (tier 0 to 3, from the receipts)

Caps are `packages/scene-assets/src/unitArt.ts` by each unit's
`vehicle_class`. The Buk-M3's unit is classed `wheeled_medium` (reported in
`decisions.md`). Every model sits at 31–63 % of its tier-0 cap. The largest
is the T-80BVM at 72,845.

| Appearance | Class | Tier 0 | 1 | 2 | 3 | Cap (0/1/2/3) | Tier 0 of cap |
|---|---|---:|---:|---:|---:|---|---:|
| `eastern_t_80_t_80bvm` | tracked_heavy | 72,845 | 21,972 | 6,161 | 2,364 | 120k/45k/13k/4k | 61 % |
| `eastern_t_15_armata_heavy_ifv` | tracked_heavy | 72,748 | 19,604 | 5,360 | 2,192 | 120k/45k/13k/4k | 61 % |
| `europe_challenger_challenger_2_tes` | tracked_heavy | 70,168 | 22,430 | 6,420 | 1,652 | 120k/45k/13k/4k | 58 % |
| `eastern_t_72_t_72b3_2016` | tracked_heavy | 68,709 | 21,466 | 5,971 | 2,268 | 120k/45k/13k/4k | 57 % |
| `eastern_t_90_t_90m_proryv` | tracked_heavy | 65,948 | 21,520 | 5,546 | 2,316 | 120k/45k/13k/4k | 55 % |
| `europe_challenger_challenger_3` | tracked_heavy | 63,092 | 18,430 | 4,818 | 1,832 | 120k/45k/13k/4k | 53 % |
| `europe_leopard_2_2a6` | tracked_heavy | 62,659 | 20,757 | 5,827 | 1,636 | 120k/45k/13k/4k | 52 % |
| `eastern_t_14_armata_main_battle_tank` | tracked_heavy | 61,044 | 20,590 | 5,858 | 2,152 | 120k/45k/13k/4k | 51 % |
| `europe_leclerc_xlr` | tracked_heavy | 61,025 | 20,903 | 5,845 | 1,904 | 120k/45k/13k/4k | 51 % |
| `europe_leopard_2_2a7v` | tracked_heavy | 60,958 | 18,672 | 5,212 | 1,760 | 120k/45k/13k/4k | 51 % |
| `europe_leopard_2_2a8` | tracked_heavy | 60,758 | 19,084 | 5,332 | 1,824 | 120k/45k/13k/4k | 51 % |
| `us_m1_abrams_sep_v3_trophy` | tracked_heavy | 57,877 | 20,279 | 5,974 | 1,840 | 120k/45k/13k/4k | 48 % |
| `us_m1_abrams_sep_v2_trophy` | tracked_heavy | 57,609 | 20,223 | 5,974 | 1,828 | 120k/45k/13k/4k | 48 % |
| `us_m1_abrams_sep_v2` | tracked_heavy | 57,281 | 19,943 | 5,854 | 1,764 | 120k/45k/13k/4k | 48 % |
| `europe_cv90120_light_tank` | tracked_medium | 53,031 | 17,243 | 5,329 | 1,792 | 90k/34k/10k/3k | 59 % |
| `europe_ajax_tracked_reconnaissance_vehicle` | tracked_medium | 51,168 | 18,228 | 5,566 | 1,580 | 90k/34k/10k/3k | 57 % |
| `us_m10_booker` | tracked_medium | 50,865 | 16,035 | 4,840 | 1,500 | 90k/34k/10k/3k | 57 % |
| `eastern_type_99_ztz_99a` | tracked_heavy | 50,298 | 15,898 | 4,586 | 1,860 | 120k/45k/13k/4k | 42 % |
| `us_m3_bradley_cfv_m3a3` | tracked_medium | 49,470 | 15,782 | 4,402 | 1,780 | 90k/34k/10k/3k | 55 % |
| `us_m2_bradley_ifv_m2a4` | tracked_medium | 48,230 | 15,374 | 4,402 | 1,756 | 90k/34k/10k/3k | 54 % |
| `us_m1e3_abrams` | tracked_heavy | 48,082 | 17,024 | 4,888 | 1,688 | 120k/45k/13k/4k | 40 % |
| `europe_pzh_2000_tracked_155_mm_howitzer` | tracked_heavy | 46,972 | 14,844 | 4,354 | 1,506 | 120k/45k/13k/4k | 39 % |
| `europe_cv90_cv9040c` | tracked_medium | 45,785 | 15,373 | 4,839 | 1,500 | 90k/34k/10k/3k | 51 % |
| `eastern_type_15_ztq_15_light_tank` | tracked_medium | 45,738 | 16,108 | 4,500 | 1,928 | 90k/34k/10k/3k | 51 % |
| `eastern_plz_05_chinese_tracked_155_mm_howitzer` | tracked_heavy | 45,208 | 14,588 | 4,262 | 1,516 | 120k/45k/13k/4k | 38 % |
| `europe_kf51_panther_prototype_main_battle_tank` | tracked_heavy | 45,040 | 15,690 | 4,560 | 1,476 | 120k/45k/13k/4k | 38 % |
| `eastern_2s19_msta_s_2s19m2` | tracked_heavy | 44,702 | 14,082 | 4,034 | 1,424 | 120k/45k/13k/4k | 37 % |
| `eastern_buk_buk_m3_medium_range_air_defense` | wheeled_medium | 43,950 | 13,908 | 4,220 | 1,384 | 70k/27k/7k/2k | 63 % |
| `us_m109_paladin_m109a7` | tracked_heavy | 42,652 | 13,676 | 3,974 | 1,416 | 120k/45k/13k/4k | 36 % |
| `europe_mars_ii_multiple_launch_rocket_system` | tracked_heavy | 40,806 | 13,416 | 3,600 | 1,048 | 120k/45k/13k/4k | 34 % |
| `us_m270_mlrs_m270a2` | tracked_heavy | 40,806 | 13,416 | 3,600 | 1,048 | 120k/45k/13k/4k | 34 % |
| `eastern_bmp_ifv_family_bmp_3` | tracked_medium | 39,547 | 15,218 | 4,269 | 1,344 | 90k/34k/10k/3k | 44 % |
| `eastern_brm_brm_3k` | tracked_medium | 39,224 | 14,634 | 4,044 | 1,316 | 90k/34k/10k/3k | 44 % |
| `europe_gepard_1a2` | tracked_heavy | 38,420 | 13,926 | 4,162 | 1,360 | 120k/45k/13k/4k | 32 % |
| `europe_puma_level_c` | tracked_heavy | 37,016 | 13,166 | 3,882 | 1,508 | 120k/45k/13k/4k | 31 % |
| `europe_cv90_mk_iv` | tracked_medium | 35,490 | 12,696 | 4,084 | 1,432 | 90k/34k/10k/3k | 39 % |
| `eastern_bmp_ifv_family_bmp_2m_berezhok` | tracked_medium | 31,544 | 12,662 | 3,360 | 1,140 | 90k/34k/10k/3k | 35 % |

### `assets/shots.txt`

- `fq` and `side` are `closeups.py`'s own `front-quarter` and `side-low` at
  800×500. Rendering the committed T-72B3 with them reproduces the planning
  closeups at distance 0.
- `battle_zoom`: 25 m (`zoom_min`, `fixtures/presentation.json:30`) at pitch
  0.22 rad (`pitch_curve`, `:33-36`), 45 degrees off the nose, at
  1920×1080. The lens is 23.9 mm, `fov_y` 0.8 rad (`:28`) at 16:9. A 10 m
  tank is about 500 px long there.
- `cinematic`: 12 m out at 35 degrees, the eye 1.5 m up, 70 mm, 1920×1080.

`closeups.py` now takes metres (a value ending `m`) for shots that keep a
real distance.

### Checks

- **Bake.** The first bake failed on `fit.hull_extents` and `fit.footprint`,
  for the `air_defence` re-export only. With that held back, the bake is
  green.
- **`asset check`.** Green: 921 runtime files match the catalog, and all
  five Blender-side Python tests pass. Catalog load is 635.4 of 640 MiB. It
  first flagged 11 unit icons as stale, the re-exported models'
  silhouettes, and `asset icons` regenerated them.
- **Narrow web test.** `web test tests/sceneAssets/unitFit.test.ts`: 5 of 5
  pass.
- **Logic tests.** `vehicle_lint_logic_test.py` passes, with the new body
  cases. The belt case fails with the rule disabled.
- **Receipts.** All 25 re-exported receipts hash the committed lint and
  family sources.
- Every export ran with the lint on and passed against the new baseline.

**Unfinished, for the coordinator.**
- The `air_defence` hook fix (lane B, slice 24).
- The Buk-M3's `wheeled_medium` class.
- `vehicle_lint.check_ground_contact` still finds tracks by `"track_" in
  name`, a second owner of "what is a track". It is harmless today: a
  `track_guard` is never a track's lowest part.
