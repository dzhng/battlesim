# 07 — Sim: ground layer rules

**Status:** done. **Depends on:** 05 (parallel with 06). **Lane:** simulation.

## Contract

An authoritative, bounded ground layer (D2). Craters give infantry partial cover, weaker than a building or forest, and slow vehicles slightly, never impassable (Q8). Scorch, track wear and trampling are recorded but have no gameplay effect.

## API seam

- `sim::ground::GroundLayer`: fixed cells of `ground.cell_m`, provisionally 1 m, each `{crater: u8, scorch: u8, tracks: u8, trampled: u8}`. Its size is bounded by the map area.
- `GroundRules` in the fixture:
  - `cell_m`;
  - crater radius per weapon, from blast radius;
  - accumulation and saturation;
  - `crater_cover`;
  - `crater_vehicle_mult`.
- Written from `BlastEvent`s and from vehicle and soldier movement.
- Crater cover joins the "strongest source wins" rule in `damage.rs:38-77`. The vehicle slowdown applies in movement integration only; navigation never reads craters (decision).
- The layer enters `Battle::digest`.

## What you can run or see

A new `/lab/ground` fixture: a shelling barrage, a tank crossing a crater field, infantry in craters, and a flat cell debug view. It is registered with a scene.

## Verification

Native tests:
- crater cover beats open ground and loses to buildings and forest;
- never impassable;
- slowdown only while moving;
- no navigation revision bump when craters appear (a counter test);
- the cosmetic channels leave combat identical;
- storage stays bounded after a 60-minute endurance run;
- digest parity.

Record frame cost in [`frame-cost.md`](../frame-cost.md) for this slice.

## Decision budget

- **Delegated:** Cell ordering, and accumulation curves within the fixture.
- Anything else this slice has to invent is a spec gap. Record it in [`choices.md`](../choices.md) and resolve it here; don't improvise.

## Must stay green

Replay and digest parity. Every existing scene and test, except the pixel checks this slice retunes: each retune is recorded in `decisions.md`, never loosened silently. `bun run check` and `bun run verify` at closeout.

## Feedback that changes this slice

If crater cover changes the village results a lot, re-tune `crater_cover` through the fixture.
