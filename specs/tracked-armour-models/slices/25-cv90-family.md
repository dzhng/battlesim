# 25: CV90 family

Follow [the family template](family-template.md). Its contract, passes, verification, delegated calls and what stays green all apply. This file adds only this family's models, the user's notes (quoted exactly) and the calls specific to this family.

**Lane:** Lane C, first. Branches from slice 16.

**Models:** CV9040C, CV90 Mk IV (`cv90.py`), CV90120 (`cv90120.py`, which imports `cv90`, `cv90120.py:29`).

## The user's notes (the done list)

- CV9040C: "front side panels sticks out".
- CV90 Mk IV: "front side panels sticks out".

Every note must visibly read as fixed in a before/after beside the reference (`assets/before/`, `assets/after/<family>/`).

## Calls specific to this family

- **The "front side panels stick out" note is this slice's to fix (C20).** Slice 09 found the cause is the CV9040C and CV90 Mk IV `armour_tiles` standing above the glacis as a square wall, not the fender. Move them onto `AR.side_skirt` with the hull `Outline`, so they stop where the glacis takes over. The CV90120 already does this, and the critique found no fin on it.

- `follow_tiles` is already replaced by `side_skirt` (slice 09). Make sure the front panels end where the glacis takes over, including the front fender (`cv90.py:81`), with `track_fender`.
- The CV90120 is judged in this slice. It has no note of its own, but must still meet the bar.

## Feedback that would change it

The user re-ranks or adds notes for these models after seeing the checkpoint shots.
