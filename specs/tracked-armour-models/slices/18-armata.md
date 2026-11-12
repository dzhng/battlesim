# 18: Armata (T-14, T-15)

Follow [the family template](family-template.md). Its contract, passes, verification, delegated calls and what stays green all apply. This file adds only this family's models, the user's notes (quoted exactly) and the calls specific to this family.

**Lane:** Lane A, after 17. Branches from the rollout plan commit.

**Models:** T-14 (`t14.py`), T-15 (`t15.py`); both import `armata.py`.

## The user's notes (the done list)

- T-15 Armata: "gun barrel is too small".

Every note must visibly read as fixed in a before/after beside the reference (`assets/before/`, `assets/after/<family>/`).

## Calls specific to this family

- Two sessions: the T-14 and the T-15 share the Armata hull, but the T-15's is reversed (engine at the front). Brief each with the other's photos as context only.
- T-15 (D4): the 30 mm cannon at its real length and shroud thickness, to the muzzle slice 15 set. The 57 mm turret stays out of scope.

## Feedback that would change it

The user re-ranks or adds notes for these models after seeing the checkpoint shots.

## Result (lane A, 2026-11-12)

Both wear Argon bodies through `scripted_body`, two Argon rounds each, with
the local edits numbered in each `assets/source/skins/<family>/argon-r2/README.md`.
Each model has lint 0 issues live and wreck, `asset validate` 0 findings,
and two exports byte-identical (R6).

| | T-14 | T-15 |
|---|---|---|
| Session | jn7fjespaej463s31fyd06zz558j2h8q | jn7f7j32g37hv6qsgc5n4w393h8j2hk0 |
| Argon cost r1 / r2 | $2.65 / $1.32 | $2.25 / $1.17 |
| Tiers (live) | 59,862 / 33,848 / 9,038 / 3,672 | 92,028 / 33,988 / 8,572 / 2,800 |
| Local edits | 3 | 3 |
| User note | (none) | 2A42 barrel 0.112 m, jacket 0.18 m, 2.2 m showing (was 0.10 m, 1.5 m) |

The T-15's second Kornet pair is fixed turret art. The frame aims one
`launcher` mount (decision LA9).

**Critique (unprimed; silhouette / surface / fittings / materials / cohesion):**

- T-14: r1 4/5/5/5/5; r2 5/5/4/4/5 (avg 4.6).
- T-15: r1 6/6/4/5/5; r2 7/7/6/5/6; r3 7/7/6/5/6 (avg 6.2).

**Below the bar, both.** What is left:

- **The shared paint look (materials):** see slice 17.
- **T-15:** the skirts read as a tiled grid, and the nose's side cheeks as
  stacked strips. Both are Argon's geometry. The module reads small; its
  height is the frame's 3.2 m.
- **T-14, the weakest of the lane:**
  - The turret is a rounded multi-facet shell where the photos show a tall,
    slab-sided box. That is a rebuild.
  - Two sloped turret-front panels read as glass.
  - The slat cages read tan although they are painted. Their rods are so
    thin that the paint's edge-wear tone covers them whole (shared-part
    request LA-S1).
  - The panoramic sight is short, held to the frame's dressing allowance
    (2.8 m + 0.3 m).
