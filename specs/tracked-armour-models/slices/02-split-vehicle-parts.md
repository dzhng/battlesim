# 02: split `vehicle_parts.py` by concept (coordinator)

## Contract

`vehicle_parts.py` (51,243 B, frozen by the ratchet) becomes eight modules,
each owning one concept. Every caller imports the owner directly, and no facade
re-exports old names. The proof is that behaviour is byte-identical.

Decision S2.

## Seam

| Module (import alias) | Owns |
|---|---|
| `vehicle_parts.py` (VP) | The kernel: the module-contract docstring (`vehicle_parts.py:1-33`), `FINE/NEAR/MID/ALL`, `TUBE_SIDES`, `place` (today's `_place`, made public because every module needs it), `wheel_node`, and `tube_part` with `_tube`/`_arc` |
| `running_gear.py` (RG) | `tyre_wheel`, `road_wheel`, `sprocket`, `idler`, `return_roller`, `_convex_hull_2d`, `track_loop`, `tracked_running_gear`, and the node contract for `wheel_*`, `track_L/R`, `track_length_m` and `link_pitch_m` |
| `hull_shape.py` (HS) | `hull_plan`, `mirror_plan`, `on_side` |
| `armour_parts.py` (AR) | `armour_tiles`, `slat_armour`, `bolted_panel`, `weld_line` |
| `roof_parts.py` (RF) | `hatch`, `cupola`, `periscope`, `sight_housing`, `roof_fittings`, `laser_warners`, `smoke_discharger_bank`, `antenna` |
| `hull_fittings.py` (HF) | `light_with_guard`, `tow_hook`, `shackle`, `cable`, `grille`, `exhaust`, `mudflap`, `fuel_tank` |
| `stowage.py` (SW) | `jerrycan`, `stowage_box`, `tarp_roll`, `cargo_bed` |
| `vehicle_guns.py` (VG) | `browning_m2`, `kord` |

`weapons.py` is not where vehicle guns go. It owns infantry weapons, in a
different frame convention (`weapons.py:1-12`).

Callers rewritten by a codemod in `throwaway/`:
- about 1,400 `VP.<fn>` call sites in about 50 roster files (wheeled and
  aircraft scripts included);
- `aircraft_parts.py:524` (the machine guns);
- `wreckage.py:503`;
- `wreckage_test.py:17`;
- `guides/`.

Imports of the tier constants (`from vehicle_parts import MID, NEAR …`) are
unchanged.

Each new module's docstring says what it owns and points back to the kernel
contract.

## Steps

1. Run the codemod: move the functions verbatim and rewrite the call sites.
2. **Probe:** an AST dump of every moved function is identical to the original.
3. Run `vehicle_lint.py -- --roster`. Its output must be identical before and
   after, issue by issue for every appearance. This run builds every family,
   aircraft included.
4. **Byte-identical exports:**
   - T-72B3, live and wreck (tracked, and `wreckage.py`);
   - Boxer (wheeled);
   - Apache (`aircraft_parts` guns).
5. Run the ratchet hook (pre-commit): every new file is under 32,000 B and
   `vehicle_parts.py` shrinks.
6. Update `packages/scene-assets/blender/README.md` so it names the modules.
7. **Push milestone.** No bake is needed, because the runtime is unchanged.

## What the human can see

Nothing visual. The module table in the blender README.

## Verification

Steps 2–5, plus the `asset check` Python tests.

## Must stay green

- every family export;
- lint output;
- `asset check` tests.

## Delegated

- import aliases;
- docstring wording;
- the codemod's implementation.

## Feedback that would change it

"Use a facade instead." That was rejected, because it gives every name two
owners (S2).

## Result (2026-11-11)

**The modules.** `vehicle_parts.py` went from 51,243 B to 4,897 B. Every module
is under the 32,000 B ratchet:

| Module | Bytes |
|---|---|
| `vehicle_parts` (kernel) | 4,897 |
| `running_gear` | 13,540 |
| `roof_parts` | 10,303 |
| `hull_fittings` | 8,952 |
| `stowage` | 6,314 |
| `armour_parts` | 4,311 |
| `vehicle_guns` | 2,876 |
| `hull_shape` | 2,770 |

The kernel exports `place` (was `_place`), `arc` (was `_arc`), `wheel_node`,
`tube_part`, `TUBE_SIDES` and the tier sets. `hull_shape` makes no objects
and imports nothing local. The aliases are `RG`, `HS`, `AR`, `RF`, `HF`, `SW`
and `VG`, beside `VP`.

**Callers.** 1,021 `VP.<fn>` call sites across 49 roster scripts became the
owner's alias, plus 8 imported names in `wreckage.py`, `wreckage_test.py` and
`aircraft_parts.py`. Comments naming `vehicle_parts.on_side` or `.hull_plan`
now name `hull_shape`. Imports of the tier sets, `tube_part` and `wheel_node`
(the aircraft modules and `guides/`) are unchanged. Normalising each alias
back to `VP.`, the caller diff has no other change.

**Proofs.**
- AST: all 42 functions dump identically to the original, with `_place` and
  `_arc` renamed on the original's tree. The kernel's constants are
  identical, and nothing is defined twice.
- Lint: `vehicle_lint.py -- --roster` prints the same 273 lines before and
  after, once the per-appearance timings are stripped. 130 appearances,
  interpenetration 3,318, floating part 226, 0 new, 0 failures. A dump of
  every issue of every appearance (3,544) is byte-identical, SHA-256
  `09fc7a64…`.
- GLBs: the slice's set was exported at HEAD and after the split, then
  compared with the committed files. All three agree on every file:

  | File | SHA-256 |
  |---|---|
  | `eastern_t_72_t_72b3_2016.glb` | `c04345c4220045d2…` |
  | `…_wreck.glb` | `22d8e446796e7350…` |
  | `…_wreck_hull.glb` | `16f890a21fda077e…` |
  | `…_wreck_turret.glb` | `06bfd48be7652d59…` |
  | `…_wreck_debris.glb` | `32d479ae772e754e…` |
  | `europe_boxer_apc.glb` | `baf149606b3826d1…` |
  | `europe_boxer_apc_wreck.glb` | `085e8f2d3dee007c…` |
  | `europe_boxer_apc_wreck_debris.glb` | `4de18bb815d2bbbd…` |
  | `ah_64e_guardian.glb` | `c731b998ab778d3b…` |
  | `us_uh_60_black_hawk_uh_60m.glb` | `344505e0150d6711…` |
  | `eastern_z_20_utility_transport.glb` | `7db084fc54343ca9…` |

  The Apache carries only a chin gun (`aircraft_parts.chin_gun`), so it never
  reaches `vehicle_guns`. The door guns do: the UH-60M's M2 and the Z-20's
  Kord (`hawk.py`). Those two were compared with their committed GLBs only,
  after the split.

- Tests: the five `asset check` Python tests pass, run as `asset check` runs
  them. Three ran under `python3` and two in Blender 5.2.1.

**For later shared slices.** New helpers go where the spec README's table
puts them: `side_skirt`, `track_fender`, `PLATE_BEVEL` and `panel_seam` in
`armour_parts`, and `gun_tube` in `vehicle_guns`. Two scopes may surprise:
- `vehicle_guns` also arms aircraft: six helicopter scripts' door guns
  (`aircraft_parts.door_gun`). The Apache's chin gun is not one of them.
- `smoke_discharger_bank` is a roof part, though many hang on turret sides.

Every receipt now names a stale `vehicle_parts.py` (see `decisions.md`).
