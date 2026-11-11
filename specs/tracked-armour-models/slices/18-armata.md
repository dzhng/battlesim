# 18: Armata (T-14, T-15)

Follow [the family template](family-template.md). Its contract, passes, verification, delegated calls and what stays green all apply. This file adds only this family's models, the user's notes (quoted exactly) and the calls specific to this family.

**Lane:** Lane A, after 17.

**Models:** T-14 (`t14.py`), T-15 (`t15.py`); both import `armata.py`.

## The user's notes (the done list)

- T-15 Armata: "gun barrel is too small".

Every note must visibly read as fixed in a before/after beside the reference (`assets/before/`, `assets/after/<family>/`).

## Calls specific to this family

- D4: keep the 30 mm cannon. Draw the barrel at its real length and shroud thickness, with `gun_tube`, to the muzzle slice 15 set. Today the art draws 1.54 m at a 5 cm radius (`t15.py:111`).
- Armata's `skirt_top`/`skirt_foot` (`armata.py:101-139`) are replaced by `side_skirt` on an `Outline` built from Armata's rings. If slice 09's probe found them non-convex, use the convex envelope.
- The T-15's 57 mm turret is out of scope.

## Feedback that would change it

The user re-ranks or adds notes for these models after seeing the checkpoint shots.
