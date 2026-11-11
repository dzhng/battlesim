# 12: closed hatches, no crew (coordinator)

## Contract

No tracked model shows crew, and every hatch is closed (user decision T5).

## Seam

- **Remove the open lid.** Delete `cupola`'s `lid_open` parameter and its open
  branch (`vehicle_parts.py:559-578`, now in `roof_parts.py`). Only tracked
  scripts call `cupola`. These callers pass `lid_open=False` and drop the
  keyword:
  - `kf51.py:185`;
  - `sp_howitzers.py:166`;
  - `t90.py:107`;
  - `type99.py:169`.
- **Delete every tracked `v.head_out(...)`:**
  - `abrams.py:144`;
  - `ajax.py:152-153`;
  - `challenger.py:79-80`;
  - `bmp.py:225`;
  - `cv90.py:223`;
  - `cv90120.py:142`;
  - `leclerc.py:103`;
  - `m10.py:222`;
  - `leopard.py:88`;
  - `t72.py:354`;
  - `t80.py:140`.
- **What stays.** `Vehicle.head_out` and `vehicle_crew.py` stay, because the
  wheeled `centauro.py:191` still uses them. That is the wheeled pass's call.

## The variable and crop

- **The one variable:** the roof.
- **The crop:** a top-down front-quarter view of the turret roof on the Abrams
  SEP v2, Leopard 2A6 and Ajax.
- **Out of scope:** everything else.

## Verification

- **Re-export** the 11 families. Abrams also re-exports M1E3.
- **Oracle:** no tracked receipt's `source_sha256` lists `vehicle_crew.py`, and
  `grep head_out` over the tracked scripts is empty.
- **Closeups**, then **`compare-screenshots`** against before.
- **Last check: an unprimed `screenshot-critique`.**

## Must stay green

The Centauro is not re-exported, and it stays unchanged.

## Delegated

None.

## Result (2026-11-11)

**The contract holds.** No tracked script places crew, and every tracked hatch
is closed. The critique agrees: on all three roofs, every hatch reads as
closed, no crew is visible, and nothing floats.

**`roof_parts.cupola`** no longer has a `lid_open` parameter. Its lid is
always closed on the ring, so its docstring no longer lists the `dark` role.
Four callers dropped `lid_open=False`: `kf51`, `sp_howitzers`, `t90` and
`type99`. Every cupola that relied on the default (`True`) now closes:
- T-72B3 and T-80BVM: the commander;
- M10: the commander;
- Challenger 2: the commander and the loader;
- Leclerc: the commander;
- BMP-3: the commander;
- Ajax: the commander and the gunner.

**`head_out`.** The grep found the slice's 11 scripts and 13 calls (Ajax and
Challenger have two each). All are deleted. Each call's locals and comments
went with it:
- Abrams: the pivot unpack;
- CV90 and Leopard: their `if` guards;
- M10: `gun_mount`;
- CV90120: `pivot`;
- Ajax, BMP, Challenger and Leopard: the unused `pivot` names.

So did every docstring that described a rider ("the commander rides head
out", "the crew in its hatches").

**Hatches open by other means.** There were two, both family-local:
- **`abrams.commander_hatch`** threw its lid rearward onto a stop. It now
  sits closed on the ring, and the stop is gone.
- **`leopard.commander_hatch`** swung its lid open. It was a near-copy of
  `RF.cupola`: the same ring, radius and height, six periscopes across the
  same arc, and the same lid. It is deleted, and the Leopard calls
  `RF.cupola("commander_cupola", …, radius=0.36)`.

Every other tracked hatch is `RF.hatch`, which is closed only.

Outside the tracked scripts, the test tank `tank.py` (test art, never game
content) still has `driver_lid_open`, its driver's lid swung up. It is not
in this lane's files. Slice 30's "`lid_open` anywhere" grep will hit it.
That is named to the coordinator.

**Oracle.**
- `grep "head_out\|lid_open\|vehicle_crew\|\.crew\b"` over the 26 tracked
  scripts in the README's scope comes back empty. The only remaining
  `head_out` call is in `centauro.py`.
- All 25 live appearances of the 16 changed scripts were exported. That is
  the 15 the slice names, plus `m1e3`, which imports `abrams`. They were
  exported with `--variant --out` to `throwaway/` and the lint on. All 25
  built, with 0 new lint issues and every tier reducing.
- The rifle-squad sources (`assets/source/roster/infantry/rifle_squad/*.glb`)
  were LFS pointers throughout. A crewed export loads its faction's soldier
  from them, so these exports never reached `vehicle_crew`.
- A `--variant` export writes no receipt, so the receipt half of the oracle
  waits for slice 16. It holds by construction: `_receipt` lists the modules
  in `sys.modules`, and `vehicle_crew` is imported only under `if v.crew`.

**Closeups.** A top-down front-quarter roof shot in metres,
`roof:4.2m,3.4m,7.4m:-0.4m,0m,2.2m:32` at 1000x700, shaded and matcap:
- **before:** the committed GLBs (`assets/after/12/before/`);
- **after:** the new exports (`after/`);
- **crops:** 3x zooms of each hatch area, before and after (`crops/`).

**`compare-screenshots`** (`assets/after/12/compare.json`, crops in
`crops.json`).
- **The whole frame barely moves.** Grayscale MAE is 0.10–0.38 out of 255,
  and edge energy is 0.91–0.99 of before.
- **The change stays in the hatch crops.** Crop MAE is 1.6–4.9, and edge
  energy is 0.59–0.89 of before: the busy crew figures are gone.
- **Boundary checks pass.** Each lid sits on its ring with no gap or float.
  Nothing else on the roof moved. The Leopard's MG3 pintle head touching
  its cupola ring is the same in the before shot.
- **Verdict:** the candidate is less wrong.

**Critique** (unprimed, `assets/after/12/critique.md`):

| Model | Silhouette | Surface | Fittings | Materials | Cohesion |
|---|---|---|---|---|---|
| Abrams SEP v2 | 6 | 6 | 4 | 5 | 5 |
| Leopard 2A6 | 5 | 5 | 5 | 5 | 5 |
| Ajax | 6 | 6 | 6 | 5 | 6 |

All three are below the family bar. None of the 15 fixes is about this
slice's variable, and each shows in the before shots too. They go to slices
21, 22 and 27, as `critique.md` lists them. One finding belongs to the shared
`cupola`'s owner: the closed lid covers the inner half of each vision block,
so the blocks read as locking tabs with glass flecks on them. The cupolas
that were already closed looked the same, so this slice kept the shared
closed geometry as it was.

**Not done here, by the wave's rules (C16).** No GLBs or receipts were
committed. Slice 16 re-exports every model. The Centauro was not touched.
`vehicle_export.py` and `vehicle_crew.py` are unchanged.
