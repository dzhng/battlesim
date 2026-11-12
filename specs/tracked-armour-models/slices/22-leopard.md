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

## Lane B result (2026-11-13)

One script for all three, `assets/source/skins/leopard/argon-r2/build.py`
(`build(variant)`), two Argon rounds, $1.10 + $2.78 = **$3.88**. Round 2 put
the PERI's mast in the turret, so the 2A7V and 2A8 bodies reach their 2.83 m
box (B9), and added the arrowhead wedge armour. Local edit 1 sweeps the turret
front back (R12, B10), with the meshes cut on every piece boundary so no face
twists. The family places the 2A8's `trophy_aps` nodes. All three lint 0 and
validate clean. The known note is fixed: the art stands at its real roof
(2.64 m 2A6, 2.80 m 2A7V/2A8 against the 2.83 m box), not the old 3.0 m.

| | silhouette | surface | fittings | materials | cohesion | mean |
|---|---|---|---|---|---|---|
| 2A6 r1 | 7 | 7 | 5 | 6 | 7 | 6.4 |
| 2A7V r1 | 6 | 7 | 5 | 6 | 6 | 6.0 |
| 2A8 r1 | 7 | 7 | 6 | 6 | 7 | 6.6 |
| 2A6 r2 | 6 | 5 | 5 | 6 | 6 | 5.6 |
| 2A7V r2 | 6 | 5 | 6 | 6 | 6 | 5.8 |
| 2A8 r2 | 6 | 4 | 6 | 6 | 6 | 5.6 |

Round 2 saw the sweep's first version, which twisted faces across its kink
("crumpled sheet"). The shipped sweep cuts on every boundary and adds no bent
face (probe in `decisions.md` B10). Below the bar, named: the roof weapons
read as boxes on posts, the 2A7V's front module reads thin, the wedge tops
are pale (shared paint), and thin rails at the hull's front corners read as
stray rods.

Argon's lofts carry a few quads up to 9.7 cm out of plane, measured before the
sweep (the sweep adds none). In the re-exported three-quarter-front and top
sheets they don't read as a crease at the battle camera, so they were left.
