# 17: T-72 family

Follow [the family template](family-template.md). Its contract, passes, verification, delegated calls and what stays green all apply. This file adds only this family's models, the user's notes (quoted exactly) and the calls specific to this family.

**Lane:** Lane A, first. Branches from slice 16.

**Models:** T-72B3, T-80BVM, T-90M, ZTZ-99A (`t72.py`; `t80.py`, `t90.py` and `type99.py` import it, so re-export all four every time `t72.py` changes).

## The user's notes (the done list)

- T-80BVM: "turret seems too small".
- T-90M: "turret seems too forward".

Every note must visibly read as fixed in a before/after beside the reference (`assets/before/`, `assets/after/<family>/`).

## Calls specific to this family

- The gun gap is already fixed by slice 08. Confirm it in this family's closeups.
- Slice 03's dome verdict applies:
  - if code won, apply it to `t72.dome_rings` (the T-80 imports it, `t80.py:32`);
  - if the skin won, slice 14 already fitted it.
- T-80BVM: rescale the turret to the survey's measured size, within the frame.
- T-90M: if the survey said the pivot was wrong, slice 15 already moved it, and the art follows. If the turret was drawn ahead of its pivot, redraw it centred on the pivot.
- `soviet_skirts` (`t72.py:146`) migrates to `side_skirt`. It may stay as a family-local function that only forwards the family's data.
- The ZTZ-99A keeps its own turret and adopts the shared helpers.

## Feedback that would change it

The user re-ranks or adds notes for these models after seeing the checkpoint shots.
