# 27: Puma and Ajax

Follow [the family template](family-template.md). Its contract, passes, verification, delegated calls and what stays green all apply. This file adds only this family's models, the user's notes (quoted exactly) and the calls specific to this family.

**Lane:** Lane C, after 26.

**Models:** Puma (`puma.py`), Ajax (`ajax.py`).

## The user's notes (the done list)

- Puma: "body shape wrong, front side panels sticks out".
- Ajax: "front side panels sticks out".

Every note must visibly read as fixed in a before/after beside the reference (`assets/before/`, `assets/after/<family>/`).

## Calls specific to this family

- Puma: rebuild the body to the survey overlay, including the MELLS as slice 15 settled it. Side modules (`puma.py:85-101`) go through `side_skirt`.
- Ajax: the side modules (`ajax.py:81-98`) go through `side_skirt`. Parent the existing RWS parts (`ajax.py:119-123`) under the new `HMG` mount's yaw and pitch nodes from slice 15, so the gun turns with its mount.
- Check the RWS on the turret roof clears the autocannon's elevation.

## Feedback that would change it

The user re-ranks or adds notes for these models after seeing the checkpoint shots.
