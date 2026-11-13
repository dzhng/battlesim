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

## R12 sweep (2026-11-13, lane C's post-merge pass)

The user's rule: every turret's front sweeps back from the mantlet (R12),
with the coordinator's floor of 0.5 m back per metre outboard (decisions
F2–F4). T-15, local edit 4 (`t15/argon-r2/README.md`): each cheek now runs
back in one straight line at 0.75 m per metre (~37°). It used to run back
13° from the gun slot and then turn steeply to the side.

- **Sweep:** the coordinator's whole-turret number stays 0.13: the
  right-hand Kornet pair (fixed turret art, LA9) stands at the outer edge.
  The turret body (`armor_paint` shell, the same formula) went 0.41 → 0.66.
- **Export:** lint 0, `asset validate` clean live and wreck, two exports
  byte-identical. Tiers 91,978 / 27,476 / 8,512 / 2,744.
- **Sheet:** [`t15-compare.png`](../assets/after/t15/t15-compare.png).

| Critique (silhouette, surface, fittings, materials, cohesion) | after edit 4 |
|---|---|
| T-15 | 6 5 5 4 6 |

The critic read the front as a wedge from above, and partly from the
three-quarter front. It named small grey squares floating beside the
turret's sides. Those were the flank bolts, which floated off the core
before this pass; they and the panel now sit on its surface, a change made
after the score. The rest is what lane A named: the low turret, the stubby
Kornets and the mottled paint (LA6, LA8). Still below the bar.
