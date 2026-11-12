# 25: CV90 family

Follow [the family template](family-template.md). Its contract, passes, verification, delegated calls and what stays green all apply. This file adds only this family's models, the user's notes (quoted exactly) and the calls specific to this family.

**Lane:** Lane C, first. Branches from the rollout plan commit.

**Models:** CV9040C, CV90 Mk IV (`cv90.py`), CV90120 (`cv90120.py`, which imports `cv90`, `cv90120.py:29`).

## The user's notes (the done list)

- CV9040C: "front side panels sticks out".
- CV90 Mk IV: "front side panels sticks out".

Every note must visibly read as fixed in a before/after beside the reference (`assets/before/`, `assets/after/<family>/`).

## Calls specific to this family

- Two sessions: the CV9040C and CV90 Mk IV as `build(variant)`, and the CV90120 (`cv90120.py` imports `cv90`; it keeps working until slice 30).
- The note is the template's flush-skirt rule: the appliqué stops where the glacis takes over (C20).
- The CV90120 has no note of its own, but must still meet the bar.

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
| CV9040C and CV90 Mk IV (`cv90 argon r1`) | `jn7eent2jvymp2dvxk3b0bwxvs8j30rk` | undelivered; model check (A5) pending |
| CV90120 (`cv90120 argon r1`) | `jn77678qa4hz4dmmja62gk8bfd8j2hbs` | undelivered; model check (A5) pending |

Each brief's vehicle part is archived as `brief.md` in its round-1 folder
(`assets/source/skins/<family>/[<vehicle>-]argon-r1/`). Ready for when it
runs: the before sheets (game renderer) of the committed
GLBs, the frame targets with the lane's trunnions, and a GLB-against-frame
measuring script (lane worktree `throwaway/argon/tools/`).
