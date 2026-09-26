# 04 — Sim: one sight shape

**Status:** planned. **Depends on:** 00. **Lane:** simulation.

## Contract

Units see farthest ahead: front 100%, sides about 50%, rear about 30%, tunable per unit kind (decision Q4+). Infantry is an even 360° (Q5). Vehicles look along the turret; the truck along its hull. Spotting, the fog sweep and the renderer all read one shape.

## API seam

- `sim::sight` owns:
  - `SightShape {front, side, rear}` from `village.json` → `sensors.sight_shape.<kind>`. Infantry kinds are pinned at `{1,1,1}`;
  - `sight_range(unit, bearing) -> f64`;
  - the forward direction: the turret bearing for the tank, the hull for the truck.
- `sensing::sees_point`, the per-target cull, and `visibility::sweep` all call it. The sweep shrinks its range per ray; masking after the fact saves nothing (landmine 7). `foliage_reach` receives the directional range.
- Sensing reads the bearing snapshot taken before this tick's fire. That snapshot is what gets published.
- Published per own unit: `OwnUnit.sight {eye[], forward, shape, range}`. It is at the published tick, for renderer fog, never interpolated.
- Scenario yaws are fixed deliberately: `UnitSetup.yaw` defaults to facing +X. `sensing.rs:27`'s pinned ranges are updated on purpose (landmine 6).

## What you can run or see

`/lab/sensors` gains a debug overlay of the sight lobe; `village_report` and `endurance_report` run before and after.

## Verification

- Native tests:
  - range at 0°, 90°, 180° and the boundary angles per kind;
  - infantry is isotropic;
  - a vehicle turned by its turret;
  - spotting and the sweep agree for one shape (`sight_shape_consumers_agree`);
  - garrison eyes;
  - digest parity.
- `observation.test.ts` round-trips `sight`.
- Record the ten-seed village report and endurance shifts in `decisions.md`. Retune the sensors scene.

Record frame cost in [`frame-cost.md`](../frame-cost.md) for this slice.

## Decision budget

- **Delegated:** Default shape values within the fixture, and how the side band is interpolated.
- **Not delegated:** Which forward direction each kind uses, and infantry isotropy. Both are settled.
- Anything else this slice has to invent is a spec gap. Record it in [`choices.md`](../choices.md) and resolve it here; don't improvise.

## Must stay green

Replay and digest parity. Every existing scene and test, except the pixel checks this slice retunes: each retune is recorded in `decisions.md`, never loosened silently. `bun run check` and `bun run verify` at closeout.

## Feedback that changes this slice

If the village's tuned encounter targets move a lot, re-tune through the fixture with paired reports. Rule changes need the user.
