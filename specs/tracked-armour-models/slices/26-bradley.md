# 26: Bradley

Follow [the family template](family-template.md). Its contract, passes, verification, delegated calls and what stays green all apply. This file adds only this family's models, the user's notes (quoted exactly) and the calls specific to this family.

**Lane:** Lane C, after 25.

**Models:** Bradley M2A4, M3A3 (`bradley.py`).

## The user's notes (the done list)

- M2A4 Bradley: "front side panels sticks out".
- M3A3 Bradley: "front side panels sticks out".

Every note must visibly read as fixed in a before/after beside the reference (`assets/before/`, `assets/after/<family>/`).

## Calls specific to this family

- **The "front side panels stick out" note is this slice's to fix (C20).** Slice 09 found the cause is the Bradley applique tiles standing above the glacis as a square wall, not the fender. Move them onto `AR.side_skirt` with the hull `Outline`, so they stop where the glacis takes over. The CV90120 already does this, and the critique found no fin on it.

- The front fender is already on `track_fender` (slice 09). The lower skirt panels (`bradley.py:102`) and applique tiles also stay inside the outline at the nose.
- Wreck selectors (`bradley.py:209`) keep addressing the panels by name.

## Feedback that would change it

The user re-ranks or adds notes for these models after seeing the checkpoint shots.
