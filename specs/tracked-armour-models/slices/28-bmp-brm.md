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

**Delayed by Bento, then run.** Round 1 was briefed and created for every vehicle at
08:53 UTC (`--tier google-recommended`, the common brief plus a vehicle
part; LC1-LC4 in `decisions.md`). Every create returned HTTP 503
`integration_unavailable`, the sessions exist but were never delivered
(`lastDeliveredInputSeq` 0), and new creates still failed the same way: Bento's
session dispatch was down workspace-wide. It recovered at about 16:00 UTC
and the sessions then ran as briefed (results below).

| Session | Bento id | State |
|---|---|---|
| BMP-2M (`bmp2m argon r1`) | `jn7d8f6c09s3cbn1ak0h74g08s8j3amn` | integrated; Argon only (A5) |
| BMP-3 and BRM-3K (`bmp3 argon r1`) | `jn7c9yatjycn23ctyqsj2xwgzs8j38gc` | integrated; Argon only (A5) |

Each brief's vehicle part is archived as `brief.md` in its round-1 folder
(`assets/source/skins/<family>/[<vehicle>-]argon-r1/`). Ready for when it
runs: the before sheets (game renderer) of the committed
GLBs, the frame targets with the lane's trunnions, and a GLB-against-frame
measuring script (lane worktree `throwaway/argon/tools/`).

### Result: all three integrated, below the bar

Bento recovered at about 16:00 UTC and every session then ran on
`google/gemini-4-argon` (plus 1-2 cents of `openai/gpt-6-luna`, Bento's own
helper, per turn).

- **Argon:** BMP-2M $2.34 + $3.23 = $5.57. BMP-3 and BRM-3K together $3.17:
  round 1 $1.13, which stopped on a provider error, plus $1.07 to continue
  it; round 2 $0.98.
- **The user's notes, fixed:** the nose stands high and the underside rises
  to it. BMP-2M nose 1.21-1.24 m (was 0.95), upper glacis about 10 degrees.
  BMP-3/BRM-3K nose 1.39-1.44 m (was 0.95-1.04), lower plate about 58
  degrees, upper glacis about 11 degrees. See the side rows of the compare
  sheets.
- **Export:** lint 0 for all three; `asset validate` clean, live and wreck
  (the wrecks under R10's 1 m rubble floor); two exports byte-identical.
- **Tiers:** all inside `tracked_medium`.
  - BMP-2M 79,694 / 30,770 / 7,845 / 2,038.
  - BMP-3 60,962 / 28,796 / 7,664 / 1,752.
  - BRM-3K 59,392 / 28,520 / 7,482 / 1,770.
- **Sheets:**
  - [`assets/after/bmp/bmp2m-compare.png`](../assets/after/bmp/bmp2m-compare.png)
  - [`assets/after/bmp/bmp3-compare.png`](../assets/after/bmp/bmp3-compare.png)
  - [`assets/after/brm/brm3k-compare.png`](../assets/after/brm/brm3k-compare.png)
- **Local edits:** BMP-2M 2, BMP-3/BRM-3K 5 (each `argon-r2/README.md`).

| Critique (silhouette, surface, fittings, materials, cohesion) | round 1 | round 2 | round 3 |
|---|---|---|---|
| BMP-2M | 6 5 5 5 6 | 5 5 4 5 6 | 6 5 5 6 6 |
| BMP-3 | 5.5 6 5 5 6 | 6 5 5 5 6 | 5 5 4 4 5 |
| BRM-3K | 5 5 4 5 5 | 6 6 5 5 6 | 6 5 4 4 6 |

Round 1 was Argon's round 2 as built. Each later round followed one set of
local edits:

- BMP-2M: round 2 after the Kornet covers came off; round 3 after the
  right-hand pair was added.
- BMP-3: round 2 after the 2A70 and the trim vane; round 3 after the taller
  sights.
- BRM-3K: round 2 after the radar housing and the plain sides.

Still below the bar. What every round kept naming:

- **The shared look:**
  - `fittings` paints tan in the Russian scheme, so Argon's large fittings
    (stowage lids, grille slats, side boxes) read as wood or cardboard;
  - the top faces wash out pale;
  - the paint's mottle reads as smudges;
  - the belt has no links (P4).
- **Turret detail Argon left plain:** the BMP-3's mantlet and sights, the
  BRM-3K's smoke tubes and searchlight, the BMP-2M's sight heads. These are
  `build.py` detail passes for slice 30's sweep.
- **Faceted shading** on the BMP-2M's bow sides; and the idlers and
  sprockets hidden under the nose and deck (BMP-3/BRM-3K).
