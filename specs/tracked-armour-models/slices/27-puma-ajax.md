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

**Delayed by Bento, then run.** Round 1 was briefed and created for every vehicle at
08:53 UTC (`--tier google-recommended`, the common brief plus a vehicle
part; LC1-LC4 in `decisions.md`). Every create returned HTTP 503
`integration_unavailable`, the sessions exist but were never delivered
(`lastDeliveredInputSeq` 0), and new creates still failed the same way: Bento's
session dispatch was down workspace-wide. It recovered at about 16:00 UTC
and the sessions then ran as briefed (results below).

| Session | Bento id | State |
|---|---|---|
| Puma (`puma argon r1`) | `jn76chmjyvrn4yy1amtxm9dres8j3fzt` | integrated; Argon only (A5) |
| Ajax (`ajax argon r1`) | `jn72xwdb9cppgxhn9ysbp1cfqx8j3efa` | integrated; Argon only (A5) |

Each brief's vehicle part is archived as `brief.md` in its round-1 folder
(`assets/source/skins/<family>/[<vehicle>-]argon-r1/`). Ready for when it
runs: the before sheets (game renderer) of the committed
GLBs, the frame targets with the lane's trunnions, and a GLB-against-frame
measuring script (lane worktree `throwaway/argon/tools/`).

### Result: both integrated, below the bar

Every turn ran on `google/gemini-4-argon` (plus 1-5 cents of
`openai/gpt-6-luna`, Bento's own helper).

- **Argon:** Puma $2.71 + $5.00 = $7.71; Ajax $2.68 + $0.61 = $3.29.
- **The user's notes, fixed:**
  - **Puma, "body shape wrong":** a single sharp prow chine at about 1.1 m
    under the long upper glacis (no vertical nose plate), and the side
    modules' upper row chamfered inward to a shoulder line, so the roof
    reads narrower than the modules.
  - **Both, "front side panels sticks out":** the front modules end flush
    with the glacis and the nose corners.
  - See the compare sheets' three-quarter and side rows.
- **Export:** lint 0 for both (under 7e6877829's corrected inside test);
  `asset validate` clean, live and wreck; two exports byte-identical.
- **Tiers:** Puma 77,110 / 29,705 / 7,411 / 2,403 (`tracked_heavy`); Ajax
  54,100 / 28,332 / 7,320 / 2,266 (`tracked_medium`).
- **Sheets:** [`assets/after/puma/puma-compare.png`](../assets/after/puma/puma-compare.png),
  [`assets/after/ajax/ajax-compare.png`](../assets/after/ajax/ajax-compare.png).
- **Local edits:** Puma 2, Ajax 1 (each `argon-r2/README.md`).

| Critique (silhouette, surface, fittings, materials, cohesion) | round 1 | round 2 |
|---|---|---|
| Puma | 4 5 4 5 5 | 5 5 4 4 5 |
| Ajax | 5 5 4 4 5 | 5 5 5 5 5 |

Round 2 followed the local edits: the Puma's PERI drum, the Ajax's smoke
banks. Stopped at two rounds; well below the bar. What every round named:

- **Turrets reading low, plain and small:** the Puma's should be a tall
  cluttered box, the Ajax's tall and angular with a raised sight box. Both
  turrets are at the frames' roofs (3.10 m and 2.57 m), so the gap is
  their shape and clutter, a rebuild beyond local edits.
- **The Ajax's nose:** the critics see a tall, near-vertical lower plate
  between the tracks in the photos, where Argon built a long glacis.
- **The shared look:** a washed-out glacis, tan `fittings` (the Ajax's deck
  panels), dark shaded sides, and the plain belt.

## The user's fixes after merge (2026-11-13)

- **Puma, "the shape is wrong, it slopes down too much":** local edit 3
  (`puma/argon-r2/README.md`). The glacis break moved from x 0.55 to 1.70,
  the prow from 1.135 m to 1.62 m, and one straight ~59° lower plate runs
  from it to the belly. The upper glacis is 2.0 m of the hull's 7.4 m (was
  3.15 m).
- **R12, every turret front swept back:** local edit 4. The Puma's cheeks
  were 8° (right, with the gunner's sight standing square across it) and
  12° (left); both are now about 25° from the mantlet's edge. The probe's
  table is in `decisions.md` (F2, F3).
- **Export:** lint 0; `asset validate` clean, live and wreck; two exports
  byte-identical. Tiers 76,238 / 25,686 / 7,366 / 2,405 (`tracked_heavy`).
- **Sheet:** [`assets/after/puma/puma-compare.png`](../assets/after/puma/puma-compare.png),
  now the merged Argon body (before) against these edits (after).

| Critique (silhouette, surface, fittings, materials, cohesion) | after edits 3–4 |
|---|---|
| Puma | 6 5 6 5 6 |

The critic read the hull's front as matching the side photos and no longer
over-sloped, and the turret as a mild wedge (swept back right of the gun,
chamfered corners). It also put the nose "at about track-top height". The
geometry says otherwise: the prow is at 1.62 m and the track top at 1.07 m,
which the game's side view shows. What it docked was what earlier rounds
named: lumpy side modules and grimy, pale paint (the shared look, LC12), a
squat turret and short sight (frame-bound, LC9), and the dark rear cages.
Still below the bar.
