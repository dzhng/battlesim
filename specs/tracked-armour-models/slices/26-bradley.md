# 26: Bradley

Follow [the family template](family-template.md). Its contract, passes, verification, delegated calls and what stays green all apply. This file adds only this family's models, the user's notes (quoted exactly) and the calls specific to this family.

**Lane:** Lane C, after 25. Branches from the rollout plan commit.

**Models:** Bradley M2A4, M3A3 (`bradley.py`).

## The user's notes (the done list)

- M2A4 Bradley: "front side panels sticks out".
- M3A3 Bradley: "front side panels sticks out".

Every note must visibly read as fixed in a before/after beside the reference (`assets/before/`, `assets/after/<family>/`).

## Calls specific to this family

- One session builds both as `build(variant)`.
- The note is the template's flush-skirt rule (C20): the appliqué tiles and lower skirt panels stay inside the hull's outline at the nose.
- Wreck selectors (`bradley.py:209`) address panels by name; `scripted_body.wreck` replaces them.

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
| M2A4 and M3A3 (`bradley argon r1`) | `jn74wyvgjabx5bxr2qkw4wy6rd8j3amq` | undelivered; model check (A5) pending |

Each brief's vehicle part is archived as `brief.md` in its round-1 folder
(`assets/source/skins/<family>/[<vehicle>-]argon-r1/`). Ready for when it
runs: the before sheets (game renderer) of the committed
GLBs, the frame targets with the lane's trunnions, and a GLB-against-frame
measuring script (lane worktree `throwaway/argon/tools/`).
