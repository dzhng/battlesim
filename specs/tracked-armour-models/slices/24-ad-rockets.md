# 24: Air defence and rockets

Follow [the family template](family-template.md). Its contract, passes, verification, delegated calls and what stays green all apply. This file adds only this family's models, the user's notes (quoted exactly) and the calls specific to this family.

**Lane:** Lane B, after 23.

**Models:** Gepard, Buk-M3 (`air_defence.py`), M270A2, MARS II (`rocket_artillery.py`).

## The user's notes (the done list)

- Buk-M3: "needs more detail".
- M270A2: "the pivot for the rocket launcher needs to go in the back".
- MARS II: "the pivot for the rocket launcher needs to go in the back".

Every note must visibly read as fixed in a before/after beside the reference (`assets/before/`, `assets/after/<family>/`).

## Calls specific to this family

- First, move the Buk into its own `roster/buk.py` (decision S14). `air_defence.py` is 26,205 B. The move also gives the Buk its own receipt.
- Buk detail: add what the references show and what reads at battle zoom. That covers the radar, the missile rails, the hull fittings and the running gear helpers.
- D3: move the turntable and elevation hinge to the rear end of the launcher box. Today they sit at `x1 - 1.0`, the front end (`rocket_artillery.py:164-168`). The launcher stays stowed on its fixed mount; raising it is out of scope.
- Siblings (re-exported and checked, never reworked):
  - Pantsir, Skyranger 30 and NASAMS ×2 come from `air_defence.py`;
  - HIMARS ×2, BM-21 and Tornado-S come from `rocket_artillery.py`;
  - `s300` is re-exported to prove it unchanged;
  - `boxer.py`, `truck_chassis.py` and `ural.py` are read-only.
- Clear the Gepard's baseline entries.

## Feedback that would change it

The user re-ranks or adds notes for these models after seeing the checkpoint shots.
