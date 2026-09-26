# Frozen spike references

The three spikes' winners, frozen so the slices that port them can prove parity instead of re-deriving behaviour. `MANIFEST.sha256` pins every file here. The verdicts, with their numbers, are in [`../../spikes/`](../../spikes/).

| Spike | Frozen reference | Identity |
|---|---|---|
| 01 renderer port | Git tag `spike-01-renderer-port-proto` (pushed), the `/lab/spike-foundation` prototype | commit `089efc2`, ported from `~/dev/game` `d4394dfc` |
| 02 sight fog | [`02/`](02/): the prototype (`web/`, `jobs/`, `driver.mjs`, metric scripts, `dump/` Rust exporter), its inputs (`data/*.json`) and perf logs | `MANIFEST.sha256` |
| 03 rig and vehicles | [`03/scripts/`](03/scripts/): the Blender scripts. [`03/out/`](03/out/): the models they built (`rifleman.blend/.glb`, `tank.glb`, `truck.glb`, in LFS) and the review sheets | `MANIFEST.sha256`; source packs by URL and sha256 in [`spikes/03.md`](../../spikes/03.md) (not redistributed here) |

## Proven behaviour → evidence → owner → parity gate

| Proven behaviour | Evidence | Owning slice | Parity gate |
|---|---|---|---|
| The pruned port renders the lit frame; overlays composite unchanged | spike 01: 0/24,946 overlay pixels differ; allocations return to baseline | 12 | **Met:** the overlay-isolation scene check (0/8,796 on the village, 0/690 on ballistics) |
| Cascades split over the map's depth range cover every camera | spike 01, landmine 2 | 13 | **Met:** the cascade-coverage test at six cameras |
| Sight-light fog: straight building edges within about 1.4 px of the casting corner, and 0.5 px of stair-stepping | spike 02 `jobs/edge.mjs`, `jobs/position.mjs` | 14 | **Met:** the fog scene (0.5 px stair; 1.40 / 1.25 px) |
| Sight-light fog agrees with the simulation's sweep | spike 02 `jobs/agree.mjs`: 0.02–0.60% | 14 | **Met:** the fog scene (0.60% street, 0.70% garrison; 5% bar) |
| Fog cost at 100 a side: 0.5–1.6 ms typical, 2.2 ms worst | spike 02 `jobs/perf.mjs` | 14 | **Met:** 0.39–0.67 ms typical, 0.96–1.55 ms worst |
| The Quaternius rig and clips: hands solved to the rifle in the rifle's frame; kneel_fire recoil lifts the muzzle 4.3 cm; the run's spine twist damped to 45%; death keeps the rifle hold | spike 03 `scripts/soldier.py`, `out/soldier_strip_*.png` | 21 | **Open:** bake the frozen `soldier.py` pipeline through the slice-11 bake, then compare joint rotations per clip frame and the hand-to-rifle distance against `out/rifleman.glb`. Only the fixes the critique named may differ, and each is recorded. |
| The tank's turret, gun, muzzle, HMG, wheels and tracks, with the muzzle on the simulation's arc (error 1e-6 m) | spike 03 `scripts/tank.py`, `out/tank.glb` | 22 | **Open:** the validator's `fit.muzzle_arc`, plus node hierarchy and pivots against `out/tank.glb`. The muzzle length changes by the user's decision (about 5.9 m). |
| The truck's deploy legs (pads land at z 0.025 m) and telescoping mast, all driven by one progress value | spike 03 `scripts/truck.py`, `out/truck_strip_deploy.png` | 22 | **Open:** the workbench's `nodes.deploy_motion` check, plus pad heights at progress 1 against `out/truck.glb` |

"Met" rows were proved by the owning slice before this freeze and are kept here for the record. "Open" rows must be met before their slice is accepted.
