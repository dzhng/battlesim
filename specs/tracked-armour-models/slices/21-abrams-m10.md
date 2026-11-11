# 21: Abrams and M10

Follow [the family template](family-template.md). Its contract, passes, verification, delegated calls and what stays green all apply. This file adds only this family's models, the user's notes (quoted exactly) and the calls specific to this family.

**Lane:** Lane B, first. Branches from slice 16.

**Models:** Abrams SEP v2, SEP v2 Trophy, SEP v3 Trophy (`abrams.py`), M1E3 (`m1e3.py`, which imports `abrams`), M10 Booker (`m10.py`).

## The user's notes (the done list)

- M1E3 Abrams: "back needs to shift up, front too pointy".
- M10 Booker: "turret shape looks wrong".

Every note must visibly read as fixed in a before/after beside the reference (`assets/before/`, `assets/after/<family>/`).

## Calls specific to this family

- Split `abrams.py` (29,056 B) by concept as the first step, before it grows (decision S14), keeping `m1e3.py:32 import abrams as A` working with no facade.
- M1E3: raise the rear deck line and blunt the nose to the survey's measurements.
- M10 (Q8c):
  - fix the turret shape against the drawing;
  - exaggerate the nose and the turret's length-to-height ratio within the frame, so it doesn't read as a small Abrams;
  - judge it in one closeup beside the SEP v2.
- Edge wear is already `chip=1.0` (slice 11).
- `abrams.py:113`'s skirt functions migrate.

## Feedback that would change it

The user re-ranks or adds notes for these models after seeing the checkpoint shots.
