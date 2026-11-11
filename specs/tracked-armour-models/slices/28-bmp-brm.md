# 28: BMP and BRM

Follow [the family template](family-template.md). Its contract, passes, verification, delegated calls and what stays green all apply. This file adds only this family's models, the user's notes (quoted exactly) and the calls specific to this family.

**Lane:** Lane C, after 27.

**Models:** BMP-2M, BMP-3 (`bmp.py`), BRM-3K (`brm.py`).

## The user's notes (the done list)

- BMP-2M: "front slopes down too much, bottom should actually slope up more".
- BRM-3K: "front slopes down too much, bottom should actually slope up more".

Every note must visibly read as fixed in a before/after beside the reference (`assets/before/`, `assets/after/<family>/`).

## Calls specific to this family

- Decision S13: the BRM-3K builds on `bmp.py`'s BMP-3 hull, the way the T-80 builds on the T-72. `brm.py` describes and redraws the same hull.
- Make the upper glacis shallower and the lower glacis steeper, against the survey's front and side overlay. This fixes the BRM by construction.
- Skirts migrate: the BMP-3's (`bmp.py:165`), and the BRM's rubber skirt and fender (`brm.py:74-75`).

## Feedback that would change it

The user re-ranks or adds notes for these models after seeing the checkpoint shots.
