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
