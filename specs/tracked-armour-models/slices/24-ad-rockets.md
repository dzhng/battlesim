# 24: Air defence and rockets

Follow [the family template](family-template.md). Its contract, passes, verification, delegated calls and what stays green all apply. This file adds only this family's models, the user's notes (quoted exactly) and the calls specific to this family.

**Lane:** Lane D, first. Branches from the rollout plan commit.

**Models:** Gepard, Buk-M3 (`air_defence.py`), M270A2, MARS II (`rocket_artillery.py`).

## The user's notes (the done list)

- Buk-M3: "needs more detail".
- M270A2: "the pivot for the rocket launcher needs to go in the back".
- MARS II: "the pivot for the rocket launcher needs to go in the back".

Every note must visibly read as fixed in a before/after beside the reference (`assets/before/`, `assets/after/<family>/`).

## Calls specific to this family

- Three sessions: the Gepard, the Buk-M3, and one for the M270A2 and MARS II as `build(variant)` (the same carrier and launcher).
- Buk: "more detail" means what the references show and what reads at battle zoom: radar, missile rails, hull fittings, running gear.
- M270A2 and MARS II (D3): the launcher's turntable and elevation hinge at the rear end of the launcher box. The launcher stays stowed on its fixed mount; raising it is out of scope.
- Siblings, re-exported and checked, never reworked: Pantsir, Skyranger 30 and NASAMS ×2 come from `air_defence.py`; HIMARS ×2, BM-21 and Tornado-S from `rocket_artillery.py`; `s300` imports `rocket_artillery`. `boxer.py`, `truck_chassis.py` and `ural.py` are read-only.
- `air_defence.py` is 26 KB: if wearing the bodies doesn't shrink it, move the Buk into its own `roster/buk.py` first (S14).

## Feedback that would change it

The user re-ranks or adds notes for these models after seeing the checkpoint shots.
