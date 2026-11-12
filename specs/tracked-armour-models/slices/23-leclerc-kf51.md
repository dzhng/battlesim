# 23: Leclerc and KF51

Follow [the family template](family-template.md). Its contract, passes, verification, delegated calls and what stays green all apply. This file adds only this family's models, the user's notes (quoted exactly) and the calls specific to this family.

**Lane:** Lane B, after 22. Branches from the rollout plan commit.

**Models:** Leclerc XLR (`leclerc.py`), KF51 (`kf51.py`).

## The user's notes (the done list)

- KF51 Panther: "turret barrel shape wrong".
- Leclerc XLR: no note. It must still meet the bar.

Every note must visibly read as fixed in a before/after beside the reference (`assets/before/`, `assets/after/<family>/`).

## Calls specific to this family

- Two sessions, one per vehicle.
- KF51: the turret and the 130 mm barrel from the survey overlay, stated in the brief as numbers.

## Feedback that would change it

The user re-ranks or adds notes for these models after seeing the checkpoint shots.

## Lane B log (2026-11-12)

**Sessions:** Leclerc `jn75q527pbvjyparz5n555s2rs8j2j25`, KF51
`jn7cfznahkd0tpse99trrgqxth8j27mx`. Created 12:52; Bento's dispatch was down
workspace-wide (`lastDeliveredInputSeq` 0), so round 1 had not started.
Cost so far: $0.

**Frame targets:** Leclerc hull 6.88 × 3.60 × 2.53, ring (0, 0), muzzle
(6.43, 0, 1.97), roof gun (-0.25, -0.58, 2.38). KF51 hull 7.80 × 3.70 × 2.50,
ring (0.30, 0), muzzle (6.40, 0, 1.95) (B4), roof gun (0.05, -0.58, 2.35).

**Before (measured):** the KF51 turret runs x -2.76..+2.92; the user's
"turret barrel shape wrong" is its wedge profile and the thin generic tube,
not its length. The Leclerc's model is 3.80 m wide against the 3.60 m frame
and 0.08 m over its roof.
