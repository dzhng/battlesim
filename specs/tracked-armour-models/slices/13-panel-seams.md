# 13: panel seams (coordinator, after 04)

## Contract

Seams and panel lines are painted in through one helper: the winner of
slice 04 (decision D1).
- If the bake won, this pass still ships grooves (decision S8).
- Families place seams in their own slices.

## Seam

`armour_parts.panel_seam(...)`, with the signature slice 04 recorded in
`decisions.md`: a polyline on a plate face, plus width and depth. The existing
per-vertex occlusion bake darkens it.

`weld_line` (a raised bead) stays, for welds the references show as raised.

## The variable and crop

- **The one variable:** whether seams read at `battle_zoom`.
- **The crop:** the T-72B3 glacis and the Leopard 2A6 turret cheek.
- **Out of scope:** everything else.

## Steps

1. Write the helper.
2. Place seams on the T-72B3 glacis as the demonstration.
3. Re-export.
4. **Closeups**, then **`compare-screenshots`** against slice 04's winning shot.
5. **Last check: an unprimed `screenshot-critique`.**

## Delegated

Internal geometry, within slice 04's dimensions.
