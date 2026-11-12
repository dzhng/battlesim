# 26: Bradley

Follow [the family template](family-template.md). Its contract, passes, verification, delegated calls and what stays green all apply. This file adds only this family's models, the user's notes (quoted exactly) and the calls specific to this family.

**Lane:** Lane C, after 25. Branches from the rollout plan commit.

**Models:** Bradley M2A4, M3A3 (`bradley.py`).

## The user's notes (the done list)

- M2A4 Bradley: "front side panels sticks out".
- M3A3 Bradley: "front side panels sticks out".

Every note must visibly read as fixed in a before/after beside the reference (`assets/before/`, `assets/after/<family>/`).

## Calls specific to this family

- One session builds both as `build(variant)`.
- The note is the template's flush-skirt rule (C20): the appliqué tiles and lower skirt panels stay inside the hull's outline at the nose.
- Wreck selectors (`bradley.py:209`) address panels by name; `scripted_body.wreck` replaces them.

## Feedback that would change it

The user re-ranks or adds notes for these models after seeing the checkpoint shots.
