# 22: Leopard 2

Follow [the family template](family-template.md). Its contract, passes, verification, delegated calls and what stays green all apply. This file adds only this family's models, the user's notes (quoted exactly) and the calls specific to this family.

**Lane:** Lane B, after 21. Branches from the rollout plan commit.

**Models:** Leopard 2A6, 2A7V, 2A8 (`leopard.py`).

## The user's notes (the done list)

- (No new note.) Known before the walk: the frame is 3.0 m tall against about 2.6 m in photos (`specs/done/unit-models/choices.md`).

Every note must visibly read as fixed in a before/after beside the reference (`assets/before/`, `assets/after/<family>/`).

## Calls specific to this family

- One session builds all three as `build(variant)`: the 2A6, 2A7V and 2A8 share the hull and turret core.
- Slice 15's roof height. The PERI sight and roof gear under `dressing_*` within the 0.3 m bulky allowance.
- The 2A8 inherits the 2A7V's frame; re-check all three.

## Feedback that would change it

The user re-ranks or adds notes for these models after seeing the checkpoint shots.

## Lane B log (2026-11-12)

**Session:** `jn769mrz3ycr34y57q2gkjg2en8j3249`, `build(variant)` for `2a6`,
`2a7v` and `2a8`. Created 12:52; Bento's dispatch was down workspace-wide
(`lastDeliveredInputSeq` 0), so round 1 had not started. Cost so far: $0.

**Frame targets:** 2A6 hull 7.70 × 3.75, roof 2.64, box top 2.70, ring
(0.30, 0), muzzle (7.12, 0, 2.05); 2A7V and 2A8 hull 8.05 (the front module),
roof 2.64 / 2.72, box top 2.83, ring (0.125, 0), muzzle (6.945, 0, 2.05);
PERI to 3.00 / 3.13 as `dressing_peri`.

**Before (measured):** both current boxes top out at 2.76 m (2A6 0.06 over
its 2.70 box), widths 3.84 against 3.75, the 2A7V 0.15 m long.
