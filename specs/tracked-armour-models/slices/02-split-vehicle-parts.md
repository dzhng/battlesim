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
