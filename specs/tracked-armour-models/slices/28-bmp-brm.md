# 28: BMP and BRM

Follow [the family template](family-template.md). Its contract, passes, verification, delegated calls and what stays green all apply. This file adds only this family's models, the user's notes (quoted exactly) and the calls specific to this family.

**Lane:** Lane C, after 27. Branches from the rollout plan commit.

**Models:** BMP-2M, BMP-3 (`bmp.py`), BRM-3K (`brm.py`).

## The user's notes (the done list)

- BMP-2M: "front slopes down too much, bottom should actually slope up more".
- BRM-3K: "front slopes down too much, bottom should actually slope up more".

Every note must visibly read as fixed in a before/after beside the reference (`assets/before/`, `assets/after/<family>/`).

## Calls specific to this family

- Two sessions: the BMP-2M, and the BMP-3 with the BRM-3K as `build(variant)` (S13: the BRM-3K is built on the BMP-3 hull).
- The note, for the brief: the upper glacis shallower and the lower glacis steeper than ours, against the survey's front and side overlay.

## Feedback that would change it

The user re-ranks or adds notes for these models after seeing the checkpoint shots.

## Lane C progress (2026-11-12)

**Blocked on Bento.** Round 1 was briefed and created for every vehicle at
08:53 UTC (`--tier google-recommended`, the common brief plus a vehicle
part; LC1-LC4 in `decisions.md`). Every create returned HTTP 503
`integration_unavailable`, the sessions exist but were never delivered
(`lastDeliveredInputSeq` 0), and new creates still failed the same way: Bento's
session dispatch was down workspace-wide. No Argon cost has been spent.

| Session | Bento id | State |
|---|---|---|
| BMP-2M (`bmp2m argon r1`) | `jn7d8f6c09s3cbn1ak0h74g08s8j3amn` | undelivered; model check (A5) pending |
| BMP-3 and BRM-3K (`bmp3 argon r1`) | `jn7c9yatjycn23ctyqsj2xwgzs8j38gc` | undelivered; model check (A5) pending |

Each brief's vehicle part is archived as `brief.md` in its round-1 folder
(`assets/source/skins/<family>/[<vehicle>-]argon-r1/`). Ready for when it
runs: the before sheets (game renderer) of the committed
GLBs, the frame targets with the lane's trunnions, and a GLB-against-frame
measuring script (lane worktree `throwaway/argon/tools/`).
