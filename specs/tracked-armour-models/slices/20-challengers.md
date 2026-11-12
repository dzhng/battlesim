# 20: Challengers

Follow [the family template](family-template.md). Its contract, passes, verification, delegated calls and what stays green all apply. This file adds only this family's models, the user's notes (quoted exactly) and the calls specific to this family.

**Lane:** Lane B, first. Branches from the rollout plan commit.

**Models:** Challenger 2 TES (`challenger.py`), Challenger 3 (`challenger_3.py`).

## The user's notes (the done list)

- Challenger 2 TES: "turret shape wrong".
- Challenger 3: "turret shape wrong".

Every note must visibly read as fixed in a before/after beside the reference (`assets/before/`, `assets/after/<family>/`).

## Calls specific to this family

- One session builds both as `build(variant)`: the Challenger 3 is a Challenger 2 hull under a new turret. If round 1 shows the hulls differ more than that (running gear, armour packs), split them, at no more than two rounds each.
- The half-width stays at the 2.1 m cap (`limits.rs:21-24`). Wider side armour packs ride as `dressing_*` within the allowance, or are named.

## Feedback that would change it

The user re-ranks or adds notes for these models after seeing the checkpoint shots.
