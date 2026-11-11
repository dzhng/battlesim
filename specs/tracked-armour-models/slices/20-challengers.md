# 20: Challengers

Follow [the family template](family-template.md). Its contract, passes, verification, delegated calls and what stays green all apply. This file adds only this family's models, the user's notes (quoted exactly) and the calls specific to this family.

**Lane:** Lane A, after 19.

**Models:** Challenger 2 TES (`challenger.py`), Challenger 3 (`challenger_3.py`).

## The user's notes (the done list)

- Challenger 2 TES: "turret shape wrong".
- Challenger 3: "turret shape wrong".

Every note must visibly read as fixed in a before/after beside the reference (`assets/before/`, `assets/after/<family>/`).

## Calls specific to this family

- The half-width stays at the 2.1 m cap (`limits.rs:21-24`). Any width the drawing asks for beyond it is drawn as dressing (side armour packs within the allowance) or named.
- Challenger 3's skirt functions (`challenger_3.py:55-67`) migrate.
- The Challenger 2's gun is already a `gun_tube` (slice 08).

## Feedback that would change it

The user re-ranks or adds notes for these models after seeing the checkpoint shots.
