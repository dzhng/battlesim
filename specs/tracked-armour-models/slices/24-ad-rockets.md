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

**Round 1 briefed (2026-11-12, 12:58 local).** One Bento session each, tier
`google-recommended`, the common brief verbatim plus the vehicle part
(`throwaway/argon/briefs/<unit>.txt` in the lane's worktree):

| Vehicle | Session | Round 1 | Round 2 | Model check (A5) |
|---|---|---|---|---|
| Gepard 1A2 | `jn74m5sakaab67mvd77fh41b5h8j343w` | not delivered | — | — |
| Buk-M3 | `jn76e3qxg3xxxm399wgbt10y858j3t1q` | not delivered | — | — |
| M270A2 + MARS II (`build(variant)`) | `jn73fbacmdh3f2p0p1vft94f5d8j234z` | not delivered | — | — |

Every create first returned 503 `integration_unavailable`; retried with the
same client request ids, each was accepted but sits undelivered
(`lastDeliveredInputSeq` 0): Bento's dispatch is down workspace-wide (the
coordinator's note). No further sessions are created until it recovers.

**What the briefs settle beyond the template** (lane D decisions D1-D4):
- Nothing on these four turns in the game: the Gepard's and Buk's mounts are
  drawn by the hull, the M270's has no role. So the bodies name no `turret`,
  `gun` or `hmg`: the turret or launcher base is `turret_body`, the weapon
  `gun_body` (the Gepard's two guns `gun_L_body`, `gun_R_body`), both under
  `hull`; the Gepard's folded search radar is `dressing_search_radar`.
- M270A2/MARS II: the turntable and the elevation hinge at the rear of the
  launcher box (hinge pin x -3.21, z 1.19, the frame's pivot), the rams
  forward of them; the box's rear face shows the pods' aft caps (the
  three-quarter-rear photo shows the box raised on that rear hinge), and the
  rockets leave through the box's front wall at x +0.50 (the frame's muzzle).
- Buk-M3: the layout is rebuilt first (no cab; the turret forward with the
  radar at its front and the six canisters lying behind it; the raised engine
  compartment at the rear; seven road wheels, rear sprocket), then the detail
  the note asks for.

**Before sheets** (game renderer, committed GLBs):
`throwaway/sheets/before/<appearance>/` in the lane's worktree.
