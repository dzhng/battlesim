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

**Delayed by Bento, then run.** Round 1 was briefed and created for every vehicle at
08:53 UTC (`--tier google-recommended`, the common brief plus a vehicle
part; LC1-LC4 in `decisions.md`). Every create returned HTTP 503
`integration_unavailable`, the sessions exist but were never delivered
(`lastDeliveredInputSeq` 0), and new creates still failed the same way: Bento's
session dispatch was down workspace-wide. It recovered at about 16:00 UTC
and the sessions then ran as briefed (results below).

| Session | Bento id | State |
|---|---|---|
| M2A4 and M3A3 (`bradley argon r1`) | `jn74wyvgjabx5bxr2qkw4wy6rd8j3amq` | integrated; Argon only (A5) |

Each brief's vehicle part is archived as `brief.md` in its round-1 folder
(`assets/source/skins/<family>/[<vehicle>-]argon-r1/`). Ready for when it
runs: the before sheets (game renderer) of the committed
GLBs, the frame targets with the lane's trunnions, and a GLB-against-frame
measuring script (lane worktree `throwaway/argon/tools/`).

### Result: both integrated, below the bar

Every turn ran on `google/gemini-4-argon`, plus about a cent of
`openai/gpt-6-luna` (Bento's own helper).

- **Argon:** round 1 $3.00 (stopped on a provider error) + $1.14 to continue
  it; round 2 $0.42 (stopped on a stream error) + $0.29 to continue it.
  Total $4.87 for both variants.
- **The user's note, fixed for both:** the applique tiles and skirt panels
  stop where the glacis takes over and stay inside the hull's outline at the
  nose. See the three-quarter and side rows of the compare sheets.
- **Export:** lint 0 for both; `asset validate` clean, live and wreck; two
  exports byte-identical. The 25 mm and the TOW box now sit on the frame's
  pivots and muzzles (Argon's round 1 had the gun 0.21 m low).
- **Tiers:** M2A4 39,808 / 26,628 / 7,368 / 1,794; M3A3 42,056 / 27,336 /
  7,600 / 1,828 (`tracked_medium`).
- **Sheets:** [`assets/after/bradley/m2a4-compare.png`](../assets/after/bradley/m2a4-compare.png),
  [`assets/after/bradley/m3a3-compare.png`](../assets/after/bradley/m3a3-compare.png).
- **Local edits:** 1 (`assets/source/skins/bradley/argon-r2/README.md`).

| Critique (silhouette, surface, fittings, materials, cohesion) | round 1 | round 2 (after the ramp door) |
|---|---|---|
| M2A4 | 5 6 5 6 6 | 6 7 5 6 6 |
| M3A3 | 5 6 5 5 5 | 5 5 4 6 6 |

Stopped at two rounds; below the bar. What every round named:

- **The turret reads low and narrow:** the frame holds its roof at 2.565 m,
  but the real turret's bulk, its bolted armour blocks and the tall CIV on
  its mast make it dominate the profile.
- **Undersized items:** the TOW box, the 25 mm (no mantlet sleeve or muzzle
  brake) and the smoke banks.
- **The glacis tiles read as planks,** and the headlights sit low.
- **The shared look:** the shaded sides cast olive, and the belt is plain.

## R12 sweep (2026-11-13)

The user's rule: every turret's front sweeps back from the mantlet (R12),
with the coordinator's floor of 0.5 m back per metre outboard (decisions
F2–F4). Local edit 3 (`bradley/argon-r2/README.md`): the box turret's
front plates angle back 0.70 m per metre (~35°) from the mantlet's edge.

- **Sweep** (coordinator's probe, cheek peak to edge): M2A4 0.18 → 0.56,
  M3A3 0.17 → 0.64. The probe folds |y| about the hull's axis and this
  turret sits 0.2 m right of it, so its right cheek decides the edge;
  measured about the turret's own axis the cheeks run back 0.70.
- **Export:** lint 0, `asset validate` clean live and wreck, two exports
  byte-identical. Tiers: M2A4 39,776 / 23,632 / 7,342 / 1,792; M3A3
  42,024 / 24,070 / 7,590 / 1,826 (`tracked_medium`).
- **Sheets:** [`m2a4-compare.png`](../assets/after/bradley/m2a4-compare.png),
  [`m3a3-compare.png`](../assets/after/bradley/m3a3-compare.png).

| Critique (silhouette, surface, fittings, materials, cohesion) | after edit 3 |
|---|---|
| M2A4 | 6 6 5 6 6 |
| M3A3 | 6 6 5 6 6 |

Both critics read the front clearly as a wedge from above, and mostly from
the three-quarter front. Both named the pale disc round the turret base.
That is the hull's ring coaming, which the swept cheeks had uncovered; it
is now 0.86 m across and under the turret, a change made after these
scores. What is left is the turret's small, low mass and the plain sights
(frame-bound, LC9). Still below the bar.
