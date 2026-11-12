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

**Round 1 briefed (2026-11-12, 12:58 local).** One Bento session each, tier
`google-recommended` (`throwaway/argon/briefs/<unit>.txt` in the lane's
worktree):

| Vehicle | Session | Round 1 | Round 2 | Model check (A5) |
|---|---|---|---|---|
| M109A7 | `jn7689cr9ja542wdqp81ff712h8j2nd9` | not delivered | — | — |
| PzH 2000 | `jn75evf0d314g4damq9m6z75es8j3e3v` | not delivered | — | — |
| 2S19M2 | `jn7073hv5r3ywdqdm1yh6m0s4h8j295s` | not delivered | — | — |
| PLZ-05 | `jn790akgaj0cn20b010s9hvp798j3z2e` | not delivered | — | — |

Bento's dispatch is down workspace-wide: the sessions were accepted but sit
undelivered (see slice 24's record).

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

**Before sheets** (game renderer, committed GLBs):
`throwaway/sheets/before/<appearance>/` in the lane's worktree.
