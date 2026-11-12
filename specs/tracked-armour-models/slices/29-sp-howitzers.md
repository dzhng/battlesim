# 29: SP howitzers

Follow [the family template](family-template.md). Its contract, passes, verification, delegated calls and what stays green all apply. This file adds only this family's models, the user's notes (quoted exactly) and the calls specific to this family.

**Lane:** Lane D, after 24. Branches from the rollout plan commit.

**Models:** M109A7, PzH 2000, 2S19M2, PLZ-05 (`sp_howitzers.py`, one script with `appearances=list(SPECS)`).

## The user's notes (the done list)

- M109A7, PzH 2000, 2S19M2, PLZ-05 (each): "turret shape looks too big, or body too small. front side panel sticks out".

Every note must visibly read as fixed in a before/after beside the reference (`assets/before/`, `assets/after/<family>/`).

## Calls specific to this family

- Four sessions, one per vehicle.
- The note, for the brief: the turret-to-hull ratio from the frame and the survey (question 3 for each card), as numbers, and the template's flush-skirt rule.

## Feedback that would change it

The user re-ranks or adds notes for these models after seeing the checkpoint shots.

## Lane D record

**Sessions and Argon cost** (tier `google-recommended`; every turn ran
`google/gemini-4-argon`, plus a cent or less of `openai/gpt-6-luna` for the
title; A5 holds):

| Vehicle | Session | Round 1 | Round 2 | Total |
|---|---|---|---|---|
| M109A7 | `jn7689cr9ja542wdqp81ff712h8j2nd9` | $2.43 | $2.17 | $4.61 |
| PzH 2000 | `jn75evf0d314g4damq9m6z75es8j3e3v` | $3.82 (stopped before its reply) | $1.73 | $5.56 |
| 2S19M2 | `jn7073hv5r3ywdqdm1yh6m0s4h8j295s` | $2.11 | $1.51 | $3.63 |
| PLZ-05 | `jn790akgaj0cn20b010s9hvp798j3z2e` | $0.59 | $0.07 | $0.67 |

**The note, as numbers in each brief** (frame and survey, question 3):

| | body length | turret (x from, to) | deck | roof | turret over deck | trunnion | world muzzle |
|---|---|---|---|---|---|---|---|
| M109A7 | 6.80 (bustle overhangs the rear plate ~0.6) | -3.40 to +1.10 | 1.64 | 2.80 | 1.16 | (0.70, 2.27) | x 6.70 |
| PzH 2000 | 8.33 (overhang ~0.4) | -4.165 to +0.60 | 1.80 | 3.06 | 1.26 | (0.60, 2.70) | x 7.70 |
| 2S19M2 | 7.15 | -2.46 to +1.53 | 1.29 | 2.82 | 1.53 | (1.35, 2.19) | x 8.34 |
| PLZ-05 | 7.30 | about -3.40 to +0.70 (unmeasured, from the quarters) | ~1.55 | ~3.0-3.1 | ~1.5 | (0.50, 2.32) | x 7.95 |

Each brief also carries the flush-skirt rule and asks for the turret against
the hull to be checked in the side render against the side photos. The guns
and turrets are hull art (no mount role): `turret_body` and `gun_body` under
`hull`, the roof machine guns `dressing_*`.

**State.** Every gun is drawn by the gun rig (D6): the turret never turns
(the mount is fixed in the hull) and the barrel pitches on the frame's pitch
node. All four: lint 0, validate clean (live and wreck), byte-identical,
tiers inside the caps, the note visibly fixed in the compare sheet (the
hull's deck and the turret's length and height now follow the numbers
above; the front skirt runs flush into the fender). `sp_howitzers.py` lost
its procedural bodies. Each folder's README numbers its local edits.

| Vehicle | Critique rounds (silhouette / surface / fittings / materials / cohesion) | Bar | Still below |
|---|---|---|---|
| [M109A7](../assets/after/sp_howitzers/m109a7-compare.png) | 6.5/7/6/5/6; 6/7/5/6/6; 7/7/6/6/6 | below (avg 6.4) | the travel lock reads as sticks, a plain brake, flat roof hatches; the paint was fixed after round 3 (8a3995b96) |
| [PzH 2000](../assets/after/sp_howitzers/pzh2000-compare.png) | 7/7/5/5/6; 7/8/6/7/7 | **meets** | the turret's angled front cheeks, a bare rear |
| [2S19M2](../assets/after/sp_howitzers/2s19m2-compare.png) | 4/6/5/5/5; 6/7/5/6/6; 5/7/5/6/5 | below | the turret's inward-leaning sides and rear overhang with the conveyor arm, the bow's upturn, the brake |
| [PLZ-05](../assets/after/sp_howitzers/plz05-compare.png) | 7/7/7/6/7; 7/7/6/6/7 | below (avg 6.6) | the turret front reads as a tank's wedge, a box-shaped brake |

The paler front decks the critics flagged on the M109A7 and PzH 2000 are
the shared paint's dust film on low upward faces, not these bodies.

**R12, swept turret fronts (2026-11-12).** Measured from above on each
turret's armour shell (gun excluded), the foremost x per |y| band:
- **M109A7: changed** (its edit 9). It was flat across ±0.82 m (x 1.01); now
  a ±0.35 m mantlet band, then faces back to x 0.76-0.86 at |y| 0.8.
- **PzH 2000:** already swept: 0.47 at the mantlet, 0.06 at |y| 0.8, -0.17 at 1.0.
- **2S19M2:** already swept: 1.28 at the mantlet, 0.88 at |y| 1.0, 0.49 at 1.6.
- **PLZ-05:** already swept: 0.71 at the mantlet, 0.28 at |y| 0.8, -0.04 at 1.0.
- The Gepard (slice 24) is already swept too: 1.41 at the tracking radar, 1.09 at |y| 0.6.
