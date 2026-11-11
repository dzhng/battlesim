# 04: painted-seam experiment (lane E2)

## Contract

A recorded verdict on how to paint seams in (decision D1), on the T-72B3 and
the Leopard 2A6. It compares two candidates:

- **grooves:** shallow geometry grooves (a strip inset into the plate) that
  the existing per-vertex occlusion bake darkens (`parts.finish`,
  `parts.py:605`).
- **bake:** a per-model baked normal+AO detail map.

The output is the signature `armour_parts.panel_seam` takes in slice 13.

## Seam

- The candidates live in `throwaway/`. Their renders and the verdict sheet go to
  `assets/experiments/04/`.
- **The bake is judged in Blender only.** The renderer samples one tiled UV per
  material layer (`models_material.wgsl:35-43`). Shipping a per-model map would
  need a second unique UV, bundle support and a shader change.
- If the bake clearly wins, this pass still ships grooves, the bake becomes a
  follow-up spec, and the user is told (decision S8).

## The variable and crop

- **The one variable:** whether seams read at the battle-zoom camera.
- **The crop:** the T-72B3 glacis and hull side, and the Leopard 2A6 turret
  cheek, at the `battle_zoom` shot.
- **Out of scope:** plate shapes, bevels, paint and proportions.

## Steps

1. Build both candidates on the two models.
2. Record the cost of each: triangles added (grooves), and bytes plus texture
   count (bake).
3. **Closeups**, then **`compare-screenshots`** against before and reference.
4. **Last check: an unprimed `screenshot-critique`**, with one round.
   - "Clearly wins" means the bake is ranked first and scores at least 1 point
     higher on surface.
   - Grooves lose if they vanish at `battle_zoom`. In that case, record a
     deeper groove and re-run once.
5. **Checkpoint,** which never blocks: `preview-shots`.

## Output

- The verdict, costs and scores go in `decisions.md`.
- The `panel_seam` signature goes to slice 13:
  - its name;
  - its arguments: a polyline on a plate face, plus width and depth;
  - its roles.

## Delegated

- groove depth and width, starting at about 1 cm wide and 6 mm deep (numbers
  are guesses);
- the bake resolution.

## Feedback that would change it

The user wants the bake despite the machinery it needs. Open the follow-up
spec now instead of queueing it.
