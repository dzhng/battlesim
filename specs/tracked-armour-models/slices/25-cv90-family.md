# 25: CV90 family

Follow [the family template](family-template.md). Its contract, passes, verification, delegated calls and what stays green all apply. This file adds only this family's models, the user's notes (quoted exactly) and the calls specific to this family.

**Lane:** Lane C, first. Branches from the rollout plan commit.

**Models:** CV9040C, CV90 Mk IV (`cv90.py`), CV90120 (`cv90120.py`, which imports `cv90`, `cv90120.py:29`).

## The user's notes (the done list)

- CV9040C: "front side panels sticks out".
- CV90 Mk IV: "front side panels sticks out".

Every note must visibly read as fixed in a before/after beside the reference (`assets/before/`, `assets/after/<family>/`).

## Calls specific to this family

- Two sessions: the CV9040C and CV90 Mk IV as `build(variant)`, and the CV90120 (`cv90120.py` imports `cv90`; it keeps working until slice 30).
- The note is the template's flush-skirt rule: the appliqué stops where the glacis takes over (C20).
- The CV90120 has no note of its own, but must still meet the bar.

## Feedback that would change it

The user re-ranks or adds notes for these models after seeing the checkpoint shots.
