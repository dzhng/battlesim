# 11: plate edges and edge wear (coordinator)

## Contract

Armour plates get a narrow, light-catching bevel in tiers 0–1 (user decision
T2). Every tracked model uses `chip=1.0` (decision Q8b).

## Seam

- **The constant.** `armour_parts.PLATE_BEVEL = 0.02` (2 cm; numbers are
  guesses).
- **Where the floor applies.**
  - `bolted_panel` uses `max(bevel, PLATE_BEVEL)`.
  - `armour_tiles` today uses `min(0.02, depth * 0.3)` (`vehicle_parts.py:376`,
    before the split). It becomes `min(max(PLATE_BEVEL, …), depth * 0.45)`.
  - `side_skirt` uses `PLATE_BEVEL`.
- **Not in `parts.py`.** It has about 890 B of ratchet room left, and a floor
  there would re-bevel every building and jet.
- **Families.** A family's own paint-role plates are raised to the floor in its
  family slice, through the template checklist.
- **Edge wear.** `abrams.py:502` gets `chip=1.0`. It takes the 0.35 default
  today (`vehicle_export.py:70`); every other tracked script already passes
  1.0. `m1e3.py` follows Abrams.

## Steps

1. **Budget probe:** apply the floor and re-export the T-80BVM, the largest
   model (72,845 triangles at tier 0 against a 120k cap). Read its receipt.
2. Re-export the Abrams family and the Leopard 2A6.
3. **Closeups**, then **`compare-screenshots`** against before.
4. **Last check: an unprimed `screenshot-critique`.**

## The variable and crop

- **The one variable:** how plate edges catch the light.
- **The crop:** fq closeups of the Abrams SEP v2 beside the Leopard 2A6, at
  `battle_zoom`.
- **Out of scope:** shapes and seams.

## Must stay green

- triangle caps;
- tiers that reduce;
- lint.

## Delegated

None.

## Feedback that would change it

The user wants a 1 cm or a 3 cm bevel instead.

## Result (2026-11-11)

Lane 09+11 (decision C16). Evidence is in `assets/after/11/`.

**As built.** `armour_parts.PLATE_BEVEL = 0.02`. The floor applies as
follows:

- `bolted_panel`: `max(bevel, PLATE_BEVEL)`, and its default `bevel` is
  `PLATE_BEVEL`.
- `armour_tiles`: `min(PLATE_BEVEL, depth * 0.45)`. That is the seam's
  `min(max(PLATE_BEVEL, min(0.02, depth * 0.3)), depth * 0.45)`
  simplified, since the inner `max` is always `PLATE_BEVEL`.
- `side_skirt`: `PLATE_BEVEL`.
- `track_fender`: `min(PLATE_BEVEL, depth * 0.45)`. The plan didn't list
  it; see `decisions.md`.

`abrams.py`'s `run(...)` now passes `chip=1.0`. `m1e3.py` already did.

**Budget probe.** The T-80BVM's triangles by tier are unchanged:
72,845 / 21,972 / 6,161 / 2,364, against the 120k cap. Bevel width moves
vertices but never adds segments (`parts._bevel_for`: two segments at
tier 0, one at tier 1, none from tier 2). So the floor costs no triangles
anywhere. On the T-80BVM it moves one part: `engine_door_plate` (0.01 →
0.02 m, vertices move up to 7 mm).

**Re-exports** (`probe-and-wear.txt`):

| Model | What changed |
|---|---|
| M1A2 SEP v2 | `cheek_plate_{L,R}` (0.015 → 0.02 m); edge wear |
| Leopard 2A6 | `side_plate_{L,R}_{0,1}` and `engine_door_{0,1}` bevels |
| T-80BVM | `engine_door_plate` only |

All three keep their triangle counts.

**Edge wear.** The wear the game turns into chips is the paint's vertex
alpha (scene-assets README, "its alpha says how worn the surface is").
Read from the tier-0 `armor_paint` vertices (`wearstat.py`):

| | mean | p90 | share over 0.3 |
|---|---|---|---|
| Abrams before | 0.121 | 0.279 | 0.062 |
| Abrams after | 0.230 | 0.493 | 0.326 |
| Leopard 2A6 | 0.234 | 0.538 | 0.327 |

So the Abrams now wears like its sibling (`wear-abrams-before-after-leopard.png`).

**Closeups and comparison** (`compare-metrics.txt`). The fq, battle_zoom
and cinematic shots are from `assets/shots.txt`. Every before/after pair
has a parity distance of 0.0012 or less, and the battle_zoom pairs 0.0003
or less.

- The studio closeups can't show the wear: Workbench draws neither the
  vertex alpha nor the game's chip threshold. That is why the wear mask
  (`wear_mask.py`) was added.
- On these two tanks the bevel change is under a centimetre, on a handful
  of plates, and is invisible at battle_zoom.
- Verdict: the candidate is less wrong, because the Abrams' wear now
  matches the roster. The light-catching edge this slice is about will
  show only once the family slices raise their own plates to the floor.

**Critique** (unprimed, last):

| Tank | Silhouette | Surface | Fittings | Materials | Cohesion |
|---|---|---|---|---|---|
| M1A2 SEP v2 | 6 | 5 | 6 | 5 | 5 |
| Leopard 2A6 | 5 | 4 | 6 | 5 | 5 |

The findings, and who owns each:

- **The shared wear recipe (coordinator, outside this lane).** Wear is
  spread across whole faces, not only edges. It is interpolated over
  large triangles, and road-wheel discs carry it across their whole face.
  The recipe is `parts.textured`'s convex-edge wear with `finish`'s
  per-vertex bake, which this slice did not touch.
- **Slices 21 and 22.**
  - The Leopard's wedge armour and the Abrams' cheek chamfer shade as
    broken triangles.
  - The two tanks share one hull.
  - Leopard edges barely read on dark camo at battle_zoom.
  - The smoke caps glow white.
- **Slice 10.** The tracks are faceted drums.

**Must stay green.**

- Triangle caps: the largest model is unchanged.
- Tiers reduce: every export checks this.
- Lint: `vehicle_lint.py --variant` reports 0 new issues on the T-80BVM,
  M1A2 SEP v2 and Leopard 2A6.
- The six `asset check` Python tests pass.

**Reach beyond these models.** The `bolted_panel` and `armour_tiles` floor
reaches every caller, wheeled ones included: Boxer, BTR, Stryker, Patriot,
S-300 and S-350, among others. Their plates under 2 cm thick re-bevel when
slice 16 re-exports them. That includes `s300`, which lane B was told may
only be re-exported "to prove it unchanged". See `decisions.md`.
