# 17: T-72 family

Follow [the family template](family-template.md). Its contract, passes, verification, delegated calls and what stays green all apply. This file adds only this family's models, the user's notes (quoted exactly) and the calls specific to this family.

**Lane:** Lane A, first. Branches from the rollout plan commit.

**Models:** T-80BVM (`t80.py`), T-90M (`t90.py`), ZTZ-99A (`type99.py`). The T-72B3 is done (the Argon pilot). All three import `t72.py`'s procedural parts; leave them there until slice 30.

## The user's notes (the done list)

- T-80BVM: "turret seems too small".
- T-90M: "turret seems too forward".

Every note must visibly read as fixed in a before/after beside the reference (`assets/before/`, `assets/after/<family>/`).

## Calls specific to this family

- Three sessions, one per vehicle: the T-80BVM, T-90M and ZTZ-99A share little hull with the T-72B3 beyond its layout.
- Give each brief the T-72B3's `argon-r2/build.py` as an example of the contract, and say its corner fix (edit 14) applies.
- T-80BVM: the turret's size from the survey (`assets/frames/t72.json`, the T-80BVM entry), stated in the brief as numbers.
- T-90M: the turret centred on the frame's pivot, stated in the brief with the pivot's x.
- The ZTZ-99A's arrowhead turret is its own; brief from its photos.

## Feedback that would change it

The user re-ranks or adds notes for these models after seeing the checkpoint shots.
