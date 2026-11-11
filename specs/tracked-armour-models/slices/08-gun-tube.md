# 08: the gun tube (coordinator)

## Contract

A tank gun is one continuous core tube, with everything else seated on it, so
no section can float or leave a see-through gap. The four copies that have the
gap today are rebuilt on it. The lint no longer hides barrel gaps.

## Seam

```python
# vehicle_guns.py
def gun_tube(name, gun, mats, start, reach, radius, muzzle_radius=None,
             sleeve=None,      # (radius, [(a, b), ...]): proud skin sections over the core
             evacuator=None,   # (x, radius, length, cap): collar over the core, capped cones
             bands=(),         # x positions of clamp bands
             muzzle=None,      # (radius_m, length_m)
             lods=ALL) -> list
"""One continuous core from `start` to `reach` along the gun node's +X; every
other piece sits on the core. Names `{name}_core|_sleeve_k|_evacuator|_band_k|
_muzzle|_bore`. Roles: paint, dark, steel, black."""
```

**Migrate in this slice,** the four copies with the gap (decision S6):
- `t72.gun_2a46` (`t72.py:208-222`), which the T-80 and T-90 import;
- `type99.main_gun` (`type99.py:175-190`), about 5 cm gaps;
- the Challenger 2's gun (`challenger.py:211-229`), about 6 cm gaps;
- the ZTQ-15's gun (`type15.py:172-182`), about 5 cm gaps.

Every other tracked gun adopts `gun_tube` in its own family slice, through the
template checklist.

**Allowlist.** The gap passed the lint through the `fume_extractor` pattern
(`vehicle_lint_allowlist.json:61`). `*_sleeve` never matches `thermal_sleeve_0`,
because `fnmatch` must match the whole name.
- After migrating, remove the `*_sleeve`, `fume_extractor` and
  `bore_evacuator` patterns (`:29,61,65-66`) if `--roster` is clean without
  them.
- Otherwise add a per-pattern `checks` scope to
  `vehicle_lint_core.filter_by_allowlist` (`:125-147`), limited to the overlap
  check, and add one logic-test case for it.
- Wheeled findings this exposes get exact baseline entries, plus a note in the
  README queue for the wheeled pass. Examples: Centauro (`centauro.py:156-162`),
  Boxer's `gun_sleeve`, the Stryker's `cannon_sleeve`.

## The variable and crop

- **The one variable:** barrel continuity.
- **The crop:** a low side view of the gun against the sky, where a gap shows.
  Shoot the T-72B3, ZTZ-99A, Challenger 2 TES and ZTQ-15.
- **Out of scope:**
  - barrel length (the frame survey owns that);
  - mantlet shapes;
  - everything else.

## Steps

1. Write `gun_tube` and migrate the four copies, keeping their current lengths
   and radii.
2. **Re-export** `t72`, `t80`, `t90`, `type99`, `challenger` and `type15`.
3. **Closeups** of the crop.
4. **`compare-screenshots`** of after against before.
5. Run `--roster` lint after narrowing the allowlist.
6. **Last check: an unprimed `screenshot-critique`** of the four guns.

## Must stay green

- frames;
- lint (apart from the wheeled entries the narrowing names);
- `asset check` tests.

## Delegated

- keyword names;
- the core's segment count per tier.

## Result (2026-11-11)

**The helper.** `vehicle_guns.gun_tube` is the seam's call, with two more
keywords: `band=(proud, length)` for the clamp bands, and `bore=` for the
bore's radius.

```python
def gun_tube(name, gun, mats, start, reach, radius, muzzle_radius=None, sleeve=None, evacuator=None, bands=(),
             band=(0.008, 0.05), muzzle=None, bore=None, lods=ALL) -> list
```

- **The core** is one cylinder from `start` to `reach`, tapering to
  `muzzle_radius`.
- **The evacuator** is turned from a profile whose ends close onto the tube
  beneath it, either the sleeve or the core. `cap > 0` closes it through cones
  (probed, but no caller uses it yet). `cap = 0` gives chamfered shoulders.
  Either way it is one piece, `{name}_evacuator`.
- **Bands** sit `proud` above the tube beneath them.
- **Detail tiers.** Bands are drawn in tiers 0–1 and the bore in tiers 0–2,
  limited to the tiers in `lods`.

**The four migrations**:
- **Name and core.** Each is called with `name="main_gun"`. Its core radius is
  the sleeve's minus 1 cm, with no taper.
- **Unchanged:** the barrel length, the sleeve and muzzle radii, the evacuator
  and band positions, and each family's own barrel root or collar.
- **Sleeves.** They now run up to the collar instead of stopping 5–8 cm
  short. The first critique read the bare core in those gaps as a pinch.
- **Muzzle reference box.** On the ZTZ-99A and Challenger 2 it now sits on the
  sleeve. It used to float 1–1.5 cm above it.

| Gun | start | core | sleeve | evacuator (r, length) | muzzle (r, length) | bore |
|---|---|---|---|---|---|---|
| `t72.gun_2a46` (T-72B3, T-80BVM, T-90M) | 0.45 | 0.082 | 0.092 | 0.145, 0.56 | 0.090, 0.18 | 0.064 |
| `type99.main_gun` | 0.50 | 0.084 | 0.094 | 0.15, 0.62 | 0.095, 0.20 | 0.064 |
| `challenger.main_gun` | 0.46 | 0.090 | 0.10 | 0.165, 0.60 | 0.105, 0.24 | 0.064 |
| `type15` (the turret's gun) | 0.45 | 0.070 | 0.080 | 0.125, 0.50 | 0.080, 0.16 | 0.055 |

**Allowlist.** `*_sleeve`, `fume_extractor` and `bore_evacuator` are removed,
with no `checks` scope. Without them, `--roster` showed seven new issues. All
seven are `floating_part`, and none is an overlap, so a scope limited to the
overlap check would have hidden nothing:

| Appearance | Part | Distance | Kind |
|---|---|---|---|
| `europe_vbci_infantry_fighting_vehicle` | `gun_sleeve` | 23.5 mm | wheeled |
| `us_m2_bradley_ifv_m2a4` | `cannon_sleeve` | 24.3 mm | tracked |
| `us_m3_bradley_cfv_m3a3` | `cannon_sleeve` | 24.3 mm | tracked |
| `us_m10_booker` | `bore_evacuator` | 34.0 mm | tracked |
| `us_m109_paladin_m109a7` | `fume_extractor` | 37.9 mm | tracked |
| `eastern_2s19_msta_s_2s19m2` | `fume_extractor` | 39.4 mm | tracked |
| `eastern_plz_05_chinese_tracked_155_mm_howitzer` | `fume_extractor` | 39.4 mm | tracked |

None of them is a real gap. Each is a closed collar around a barrel that runs
through it. The floating check samples vertices, edge midpoints and face
centres, and never counts an intersection as contact (see `decisions.md`).
All seven got exact baseline entries. The Centauro, Boxer and Stryker
examples did not surface. The README's queue already tells the wheeled pass
to clear these entries.

**Proofs.**
- **Exports.** The T-72B3, T-80BVM, T-90M, ZTZ-99A, Challenger 2 TES and
  ZTQ-15 export to `throwaway/` with lint on, with 0 new issues each.
- **Roster lint.** `vehicle_lint.py -- --roster` linted 130 appearances with 0
  new issues and 0 failures. Its summary counts issues before the baseline and
  allowlist filter them. Interpenetration is unchanged at 3,318 (slice 02's
  count). Floating parts went from 226 to 220: the six tanks each lost the
  floating `fume_extractor` the allowlist used to hide, and the lathed collar
  touches its tube.
- **Tests.** The five `asset check` Python tests pass.
- **Frames.** They are untouched: every gun still reaches its frame's muzzle.

**The crop.** The shot is `--shot gunside:0.78,1.7,0.35:0.78,0.5,0.63:45 --res
1600x800`. The before shots are the committed GLBs, and the after shots are the
throwaway exports. Checkpoint shots are in `assets/after/08/`:
- `gunside-before.png` and `gunside-after.png` are 2×2 sheets;
- `<model>-before-shaded`, `-after-shaded` and `-after-matcap` are 2× crops of
  the barrel.

**compare-screenshots, after against before.** Pixelmatch puts every
difference in the evacuator gaps, the collar's shoulders and the reference
box. Nothing else on any model moved.

| Shot | distance | mae | diffRatio32 | pixelmatch | edgeEnergyRatio |
|---|---|---|---|---|---|
| T-72B3 shaded / matcap | 0.0013 / 0.0010 | 0.11 / 0.05 | 0.0006 / 0.0005 | 0.0008 / 0.0006 | 0.997 / 1.002 |
| ZTZ-99A shaded / matcap | 0.0022 / 0.0008 | 0.07 / 0.04 | 0.0004 / 0.0004 | 0.0005 / 0.0005 | 0.992 / 0.999 |
| Challenger 2 shaded / matcap | 0.0022 / 0.0006 | 0.12 / 0.05 | 0.0007 / 0.0004 | 0.0009 / 0.0006 | 0.993 / 1.000 |
| ZTQ-15 shaded / matcap | 0.0028 / 0.0020 | 0.11 / 0.05 | 0.0005 / 0.0003 | 0.0011 / 0.0008 | 0.991 / 0.994 |

The verdict: the candidate is less wrong. The baseline shows sky through the
barrel at both ends of every evacuator, and the candidate is continuous.

**Critique** (unprimed, two rounds; the receipts check found no unbacked
numbers):

| Round | Silhouette | Surface | Fittings | Materials | Cohesion |
|---|---|---|---|---|---|
| 1 | 6 | 5 | 4 | 5 | 7 |
| 2 (last check) | 5 | 6 | 4 | 4 | 5 |

- **Fixed after round 1:**
  - the bare core showing either side of each evacuator, which read as
    pinched;
  - the evacuator's highlight smearing along the collar;
  - the floating muzzle reference box.
- **Round 2 confirms the slice's variable:** "all barrels straight and
  continuous", with no detached segments, z-fighting or faceting.
- **Its other findings are outside this slice's one variable**, and are routed
  to the family slices in `decisions.md`:
  - the missing ZTZ-99A mantlet;
  - the Challenger's butt joint;
  - camouflage stretched along the barrels;
  - no taper;
  - plain evacuator cans;
  - no bands on the ZTQ-15;
  - the placeholder sensor cube;
  - the root joins.

  The scores sit below the family bar for those reasons. The bar is judged
  per family in slices 17, 19 and 20.
