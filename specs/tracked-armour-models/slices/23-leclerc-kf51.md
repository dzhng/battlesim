# 23: Leclerc and KF51

Follow [the family template](family-template.md). Its contract, passes, verification, delegated calls and what stays green all apply. This file adds only this family's models, the user's notes (quoted exactly) and the calls specific to this family.

**Lane:** Lane B, after 22.

**Models:** Leclerc XLR (`leclerc.py`), KF51 (`kf51.py`).

## The user's notes (the done list)

- KF51 Panther: "turret barrel shape wrong".
- Leclerc XLR: no note. It must still meet the bar.

Every note must visibly read as fixed in a before/after beside the reference (`assets/before/`, `assets/after/<family>/`).

## Calls specific to this family

- KF51: rebuild the turret and the 130 mm barrel against the survey overlay, with `gun_tube`.
- `leclerc.py:86`'s skirt functions migrate.

## Feedback that would change it

The user re-ranks or adds notes for these models after seeing the checkpoint shots.
