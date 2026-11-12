# 27: Puma and Ajax

Follow [the family template](family-template.md). Its contract, passes, verification, delegated calls and what stays green all apply. This file adds only this family's models, the user's notes (quoted exactly) and the calls specific to this family.

**Lane:** Lane C, after 26. Branches from the rollout plan commit.

**Models:** Puma (`puma.py`), Ajax (`ajax.py`).

## The user's notes (the done list)

- Puma: "body shape wrong, front side panels sticks out".
- Ajax: "front side panels sticks out".

Every note must visibly read as fixed in a before/after beside the reference (`assets/before/`, `assets/after/<family>/`).

## Calls specific to this family

- Two sessions, one per vehicle.
- Puma: the body from the survey overlay, the MELLS as slice 15 settled it. The side modules follow the flush-skirt rule.
- Ajax: the RWS is the `hmg` / `hmg_gun` pair on slice 15's `HMG` mount (`on: autocannon`), and it clears the autocannon's elevation.

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
| Puma (`puma argon r1`) | `jn76chmjyvrn4yy1amtxm9dres8j3fzt` | undelivered; model check (A5) pending |
| Ajax (`ajax argon r1`) | `jn72xwdb9cppgxhn9ysbp1cfqx8j3efa` | undelivered; model check (A5) pending |

Each brief's vehicle part is archived as `brief.md` in its round-1 folder
(`assets/source/skins/<family>/[<vehicle>-]argon-r1/`). Ready for when it
runs: the before sheets (game renderer) of the committed
GLBs, the frame targets with the lane's trunnions, and a GLB-against-frame
measuring script (lane worktree `throwaway/argon/tools/`).
