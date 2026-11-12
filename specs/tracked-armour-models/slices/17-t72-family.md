# 17: T-72 family

Follow [the family template](family-template.md). Its contract, passes, verification, delegated calls and what stays green all apply. This file adds only this family's models, the user's notes (quoted exactly) and the calls specific to this family.

**Lane:** Lane A, first. Branches from the rollout plan commit.

**Models:** T-80BVM (`t80.py`), T-90M (`t90.py`), ZTZ-99A (`type99.py`). The T-72B3 is done (the Argon pilot). All three import `t72.py`'s procedural parts; leave them there until slice 30.

## The user's notes (the done list)

- T-80BVM: "turret seems too small".
- T-90M: "turret seems too forward".

Every note must visibly read as fixed in a before/after beside the reference (`assets/before/`, `assets/after/<family>/`).

## Calls specific to this family

- Three sessions, one per vehicle: the T-80BVM, T-90M and ZTZ-99A share little hull with the T-72B3 beyond its layout.
- Give each brief the T-72B3's `argon-r2/build.py` as an example of the contract, and say its corner fix (edit 14) applies.
- T-80BVM: the turret's size from the survey (`assets/frames/t72.json`, the T-80BVM entry), stated in the brief as numbers.
- T-90M: the turret centred on the frame's pivot, stated in the brief with the pivot's x.
- The ZTZ-99A's arrowhead turret is its own; brief from its photos.

## Feedback that would change it

The user re-ranks or adds notes for these models after seeing the checkpoint shots.

## Result (lane A, 2026-11-12)

All three wear Argon bodies through `scripted_body`, two Argon rounds each,
local edits numbered in each `assets/source/skins/<family>/argon-r2/README.md`.
Every model: lint 0 issues live and wreck, `asset validate` 0 findings, two
exports byte-identical (R6). Compare sheets in `assets/after/<family>/`.

| | T-80BVM | T-90M | ZTZ-99A |
|---|---|---|---|
| Session | jn7apfv3bda3bpmk1mj6jxqhcx8j2pyw | jn71k7ab1h76cs7dmeqg5f9vrx8j2yga | jn78ttc97afrpveddsafyh0kd98j3x2a |
| Argon cost r1 / r2 | $1.95 / $2.18 | $2.40 / $2.27 | $1.15 / $1.24 |
| Tiers (live) | 70,212 / 31,302 / 7,342 / 2,586 | 62,980 / 30,380 / 9,614 / 3,002 | 73,434 / 32,374 / 8,828 / 3,080 |
| Local edits | 1 | 3 | 3 |
| User note | turret 3.40 x 3.45 m (was 3.38 x 3.15) | turret front x = +1.52, bustle to -3.0 (front was +2.26) | (none) |

Costs include a few cents a turn of the session's helper model
(`openai/gpt-6-luna`); Argon built every round (decision LA2).

**Critique (unprimed; silhouette / surface / fittings / materials / cohesion):**

- T-80BVM: r1 6/6/5/5/6; r2 6/6/6/5/6 (avg 5.8).
- T-90M: r1 5/6/5/5/5; r2 5/7/6/5/6 (avg 5.8).
- ZTZ-99A: r1 5/7/6/5/6; r2 7/7/6/5/6; r3 7/7/7/5/6 (avg 6.4).

**Below the bar, all three.** What is left:

- **Materials (5) is the shared paint look**, as in the pilot. The critics
  read the scheme's mottle and dust as bleached glacis tops and the
  workbench's side light as near-black flanks. Measured, the new bodies are
  no darker than the procedural ones in the same shots (lane decision LA6).
- **T-90M:** the critic wants the turret moved forward and taller. Moving it
  forward would undo the user's note and the drawing. Its height is the
  frame's 2.22 m.
- **T-80BVM:** the critic reads the cast dome ringed by tilted Relikt tiles
  as an older Kontakt turret. It wants flat blunt blocks, which means
  rebuilding the turret front (more Argon or hand modelling). Its smoke
  launchers don't read.
- **ZTZ-99A:** the turret still reads a little low. Its roof is bound by the
  frame's 2.5 m top and 0.1 m tolerance, and the roof gun's 2.21 m pivot.
