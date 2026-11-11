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
