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
