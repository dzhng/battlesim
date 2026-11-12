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

## Lane B result (2026-11-13)

**KF51:** `assets/source/skins/kf51/argon-r2/`, two rounds, $1.70 + $1.42 =
**$3.12**. Four local edits: the arrowhead's prow blunted round a proud
mantlet, a raised launcher box, a larger commander's sight, and the front swept
back (R12). The 130 mm is a thick sleeve with a heavy muzzle, and the turret
a long flat faceted wedge over the glacis, so the user's note ("turret
barrel shape wrong") reads as fixed in `assets/after/kf51/kf51-compare.png`.
Lint 0, validate clean.

| | silhouette | surface | fittings | materials | cohesion | mean |
|---|---|---|---|---|---|---|
| KF51 r1 | 6 | 7 | 7 | 6 | 7 | 6.6 |
| KF51 r2 | 7 | 7 | 6 | 6 | 7 | 6.6 |
| KF51 r3 | 7 | 7 | 6 | 6 | 7 | 6.6 |

Below the bar, named: the commander's sight can't grow taller within the
0.3 m roof allowance; there's a pale glare on one cheek (shared); the turret
reads a little tall-sided.

**Leclerc XLR:** `assets/source/skins/leclerc/argon-r1/`, one round,
**$2.46**. Round 2 wasn't needed (B7). Three local edits: the armour blocks painted as
armour, the cheek corners filled, and the cheek fronts swept back instead of
caving in (the user's note: "why does the side cave outwards?"). Lint 0,
validate clean, byte-identical twice.

| | silhouette | surface | fittings | materials | cohesion | mean |
|---|---|---|---|---|---|---|
| Leclerc r1 | 7 | 7 | 6.5 | 6 | 6.5 | 6.6 |
| Leclerc r2 | 6 | 6 | 6.5 | 6 | 6 | 6.1 |
| Leclerc r3 (cheeks swept) | 6 | 7 | 6 | 6 | 6 | 6.2 |

Below the bar, named: the right cheek's appliqué reads as a flat slab, the
road wheels read brassy, and the turret and engine deck merge into one long
box from the side.

**Leclerc R12 sweep (2026-11-13):** the turret front measured 0.42 m back per
metre outboard, under rule R12's floor of 0.5. The cheek yaw alone couldn't
reach it: the shell's nose corner stood forward at the turret's edge. Edit 4
(folder README) ends the build with lane B's `sweep_turret_front` (B10),
0.25 per metre more on the cheeks, shell nose and corner blocks together. It
now measures 0.62. See `assets/after/leclerc/leclerc-compare.png`. Lint 0, validate clean
live and wreck, byte-identical twice. Unprimed critique of the after shots:

| | silhouette | surface | fittings | materials | cohesion | mean |
|---|---|---|---|---|---|---|
| Leclerc XLR (R12) | 6 | 6 | 6 | 5 | 6 | 5.8 |

Below the bar, named. The critic's first finding is that the front is
"swept too far back; the Leclerc's is a broad, nearly flat face". That is
the user's R12 note reversed, and the note was made on this tank, so it
stands. Also named, all present before the edit: hard-edged dark facets on
the far cheek block (Argon's tapered block, in the before shot too), the
camouflage breaking at each skirt panel, the roof weapon station's gun not
reading, the appliqué reading as a stuck-on slab, and the dark shadow side
(the shared lighting).
