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

**Delayed by Bento, then run.** Round 1 was briefed and created for every vehicle at
08:53 UTC (`--tier google-recommended`, the common brief plus a vehicle
part; LC1-LC4 in `decisions.md`). Every create returned HTTP 503
`integration_unavailable`, the sessions exist but were never delivered
(`lastDeliveredInputSeq` 0), and new creates still failed the same way: Bento's
session dispatch was down workspace-wide. It recovered at about 16:00 UTC
and the sessions then ran as briefed (results below).

| Session | Bento id | State |
|---|---|---|
| CV9040C and CV90 Mk IV (`cv90 argon r1`) | `jn7eent2jvymp2dvxk3b0bwxvs8j30rk` | integrated; Argon only (A5) |
| CV90120 (`cv90120 argon r1`) | `jn77678qa4hz4dmmja62gk8bfd8j2hbs` | integrated; Argon only (A5) |

Each brief's vehicle part is archived as `brief.md` in its round-1 folder
(`assets/source/skins/<family>/[<vehicle>-]argon-r1/`). Ready for when it
runs: the before sheets (game renderer) of the committed
GLBs, the frame targets with the lane's trunnions, and a GLB-against-frame
measuring script (lane worktree `throwaway/argon/tools/`).

### CV90120: integrated, below the bar

- **Argon:** round 1 $1.34, round 2 $1.88 (`google/gemini-4-argon`, plus
  1-2 cents of `openai/gpt-6-luna`, Bento's own helper, on every turn).
- **Export:** lint 0 issues; `asset validate` clean, live and wreck; tiers
  52,232 / 26,682 / 7,510 / 2,334 (`tracked_medium` caps 90k / 34k / 10k /
  3k); two exports byte-identical.
- **Sheet:** [`assets/after/cv90/cv90120-compare.png`](../assets/after/cv90/cv90120-compare.png).
- **Local edits:** 2 (`assets/source/skins/cv90120/argon-r2/README.md`).

| Critique round | silhouette | surface | fittings | materials | cohesion |
|---|---|---|---|---|---|
| 1 (Argon round 2 as built) | 6 | 5 | 5 | 4 | 6 |
| 2 (after edit 1, the gun) | 6 | 5 | 6 | 5 | 6 |
| 3 (after edit 2, the mantlet) | 6 | 5 | 5 | 5 | 6 |

Still below the bar (average 5.4). What the critics kept naming:

- **The turret reads low and set back.** Its roof (2.40 m) and ring
  (0.3 m behind the middle) are the frame's (P9); not moved.
- **Washed-out camouflage on the top faces, metal and rubber reading
  alike:** the shared paint and specular look (the pilot's "not ours here").
- **A bare-looking rear and bow at close range:** the rear door, lamps and
  headlight clusters are built but small; the workbench's rear view is
  backlit. A `build.py` detail pass for the lane's sweep.
- After round 3 the canvas cover and bumper moved from `fittings` (which
  paints tan in the Swedish scheme) to `rubber`; not re-critiqued.

### CV9040C and CV90 Mk IV: integrated, below the bar

- **Argon:** $2.06 + $0.24 = $2.30 for both variants. Round 2's turn ended
  on a provider error after writing all its outputs.
- **The user's note, fixed for both:** the side armour stops where the
  glacis takes over, its top following the glacis line down, flush with the
  fender. See the three-quarter and side rows of the sheets.
- **Export:** lint 0 for both, from round 1 on; `asset validate` clean, live
  and wreck; two exports byte-identical.
- **Tiers:** CV9040C 53,190 / 26,428 / 6,702 / 2,074; Mk IV 45,738 / 25,656 /
  6,550 / 2,006 (`tracked_medium`).
- **Sheets:** [`assets/after/cv90/cv9040c-compare.png`](../assets/after/cv90/cv9040c-compare.png),
  [`assets/after/cv90/mk_iv-compare.png`](../assets/after/cv90/mk_iv-compare.png).
- **Local edits:** 2 (`assets/source/skins/cv90/argon-r2/README.md`).

| Critique (silhouette, surface, fittings, materials, cohesion) | round 1 | round 2 (after the lamps and the drum sight) |
|---|---|---|
| CV9040C | 6 6 5 5 6 | 5 6 5 5 6 |
| CV90 Mk IV | 6 5 5 5 6 | 6 5 5 5 6 |

Stopped at two rounds; still below the bar. What the critics named:

- **The Mk IV's sight mast and Spike pod read short and flat:** both already
  stand at the frame's limits (the sight at the 0.3 m dressing allowance
  over its 2.8 m box, the pod at the box's top).
- **The CV9040C's drum sight reads small.**
- **The nose reads boxy with no front cheeks or mudguards.**
- **The shared look:** the splinter print's stair-stepped edges at 2x, a
  washed-out glacis, tan `fittings`, and the plain belt.
