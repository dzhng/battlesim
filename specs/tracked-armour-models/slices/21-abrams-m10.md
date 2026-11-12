# 21: Abrams and M10

Follow [the family template](family-template.md). Its contract, passes, verification, delegated calls and what stays green all apply. This file adds only this family's models, the user's notes (quoted exactly) and the calls specific to this family.

**Lane:** Lane B, after 20. Branches from the rollout plan commit.

**Models:** Abrams SEP v2 Trophy and SEP v3 Trophy (`abrams.py`), M1E3 (`m1e3.py`), M10 Booker (`m10.py`). The SEP v2 is done (the Argon pilot).

## The user's notes (the done list)

- M1E3 Abrams: "back needs to shift up, front too pointy".
- M10 Booker: "turret shape looks wrong".

Every note must visibly read as fixed in a before/after beside the reference (`assets/before/`, `assets/after/<family>/`).

## Calls specific to this family

- **The Abrams variants (P12):** one new session gets the SEP v2's `argon-r2/build.py` and adds `build(variant)` for the SEP v2 Trophy and SEP v3 Trophy: Trophy housings in place of the bustle's side stowage and rails; the SEP v3's UAPU box, CROWS-LP and data link. The SEP v2's own geometry must come back unchanged. Its rounds archive in `skins/abrams/argon-variants-r{1,2}/`, and the family then wears the one script for all three.
- M1E3: its own session; the user's note in the brief as numbers (rear deck line raised, nose blunter) against the frame.
- M10 (Q8c): its own session. The turret shape from the drawing, and a nose and turret length-to-height ratio that doesn't read as a small Abrams. Judge it in one sheet beside the SEP v2.

## Feedback that would change it

The user re-ranks or adds notes for these models after seeing the checkpoint shots.
