# 24: Air defence and rockets

Follow [the family template](family-template.md). Its contract, passes, verification, delegated calls and what stays green all apply. This file adds only this family's models, the user's notes (quoted exactly) and the calls specific to this family.

**Lane:** Lane D, first. Branches from the rollout plan commit.

**Models:** Gepard, Buk-M3 (`air_defence.py`), M270A2, MARS II (`rocket_artillery.py`).

## The user's notes (the done list)

- Buk-M3: "needs more detail".
- M270A2: "the pivot for the rocket launcher needs to go in the back".
- MARS II: "the pivot for the rocket launcher needs to go in the back".

Every note must visibly read as fixed in a before/after beside the reference (`assets/before/`, `assets/after/<family>/`).

## Calls specific to this family

- Three sessions: the Gepard, the Buk-M3, and one for the M270A2 and MARS II as `build(variant)` (the same carrier and launcher).
- Buk: "more detail" means what the references show and what reads at battle zoom: radar, missile rails, hull fittings, running gear.
- M270A2 and MARS II (D3): the launcher's turntable and elevation hinge at the rear end of the launcher box. The launcher stays stowed on its fixed mount; raising it is out of scope.
- Siblings, re-exported and checked, never reworked: Pantsir, Skyranger 30 and NASAMS ×2 come from `air_defence.py`; HIMARS ×2, BM-21 and Tornado-S from `rocket_artillery.py`; `s300` imports `rocket_artillery`. `boxer.py`, `truck_chassis.py` and `ural.py` are read-only.
- `air_defence.py` is 26 KB: if wearing the bodies doesn't shrink it, move the Buk into its own `roster/buk.py` first (S14).

## Feedback that would change it

The user re-ranks or adds notes for these models after seeing the checkpoint shots.

## Lane D record

**Sessions and Argon cost** (tier `google-recommended`; every turn's
`usageByModel` was `google/gemini-4-argon`, plus a cent or less of
`openai/gpt-6-luna` for the session title, as in the pilot; A5 holds).
Bento's dispatch was down for three hours after the creates; the sessions
were delivered at 15:57 local.

| Vehicle | Session | Round 1 | Round 2 | Total |
|---|---|---|---|---|
| Gepard 1A2 | `jn74m5sakaab67mvd77fh41b5h8j343w` | $3.74 | $1.23 | $4.97 |
| Buk-M3 | `jn76e3qxg3xxxm399wgbt10y858j3t1q` | $0.26 (provider error) + $1.99 (finished, D5) | $0.90 | $3.15 |
| M270A2 + MARS II | `jn73fbacmdh3f2p0p1vft94f5d8j234z` | $1.65 | $2.73 (provider error after writing its outputs, D5) | $4.38 |

**State** (each folder's README numbers its local edits):

- **M270A2 and MARS II: done to the export bar, below the critique bar.**
  One script, a variant each (`scripted_body.wear(..., variant=)`). The
  user's note is fixed: the launcher box hinges at the rear of the box (pin
  at x -3.21, z 1.19, the frame's pivot), its turntable under the rear;
  [`m270a2-hinge.png`](../assets/after/rocket_artillery/m270a2-hinge.png)
  swings the box 35 degrees on that origin, front end over the cab. Lint 0,
  validate clean, byte-identical; compare sheets
  [`m270a2`](../assets/after/rocket_artillery/m270a2-compare.png),
  [`mars_ii`](../assets/after/rocket_artillery/mars_ii-compare.png).
  Critique round 1 / 2: M270A2 7/7/6/6/5, 7/7/5/7/7; MARS II
  7/6.5/6.5/6/6.5, 7/7/6/6/6. Still below: the bare hull nose (no tow
  hooks, guarded lights or fender lips), the plain cab roof and sides, and
  the box's side panels (edit 4 after round 2 paints them).
- **Gepard 1A2: done to the export bar, below the critique bar.** Its gun
  mount is now drawn by the gun rig (D6): the turret turns, both guns
  pitch as one `gun`, the folded search radar rides the turret. Its wreck
  takes a 1.2 m footprint allowance (D7). Lint 0, validate clean,
  byte-identical; [`gepard-compare.png`](../assets/after/air_defence/gepard-compare.png).
  Critique 5/7/6/6/6, then 6/7/6/6/6. Still below: the turret reads low and
  flat (its roof is the frame's 2.76 m), the tracking radar small, the
  folded search radar box-like.
- **Buk-M3: integrated, not committed; blocked on its tier-1 budget.** The
  layout is rebuilt (no cab; the turret forward with the 9S36M "pillow"
  radar at its front and six canisters behind; the raised engine
  compartment at the rear; seven road wheels, rear sprocket) and the note
  "needs more detail" reads as fixed:
  [`buk-compare.png`](../assets/after/air_defence/buk-compare.png). Lint 0,
  byte-identical, wreck validates; the live GLB's tiers are 67.4k / 36.3k /
  9.7k / 2.5k against `tracked_medium`'s 90k / 34k / 10k / 3k (after R11),
  so tier 1 is 2.3k over. The coordinator holds the call. Critique round 1
  (before edit 1 painted the radar): 6/6/6/5/5.5.
- `air_defence.py` was not split (S14): wearing the bodies removes
  `gepard()` now and `tracked_hull()`/`buk()` with the Buk, about 170
  lines. Pantsir, Skyranger 30 and NASAMS are untouched.
- Siblings: HIMARS x2 and the S-300 were re-exported with the rocket
  family; their committed GLBs were stale against the shared modules (the
  committed script exports the same bytes), 0 new lint issues, validate
  clean. BM-21 and Tornado-S came out byte-identical.
