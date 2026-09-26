# 05 — Sim: animation feed

**Status:** planned. **Depends on:** 04. **Lane:** simulation.

## Contract

Each side receives everything needed to pose its own and visible enemy models, and to place effects. The simulation never names animations (decision Q7).

## API seam

`contract::observation` gains:
- **Member ids:** `OwnUnit.member_ids` and `IdentifiedUnit.member_ids`. These are the raw `Soldier.id`; the leak is accepted (F2).
- **Weapon pose:** `WeaponPose {mount, bearing, elevation, shots}` for every own mount, and for mounts of identified enemies. The tank has two (cannon and HMG). New state enters `Mount::digest`.
- **Blasts:** `ObservationFrame.blasts[{point, radius, kind}]`, clipped to seen ground.
- **Segments:** `VisibleSegment` gains:
  - `kind` (round kind; revealing the shooter's class is wanted, F3);
  - `shooter_member`;
  - `impact_normal`;
  - `hit: Ground|Hull|Prop|Soldier|None`.
- **Corpses:** `Corpse` gains `soldier`, `kind` and `yaw`.
- **Encoding:** integers go into 16-bit limbs inside the float32 transport, exact beyond 2²⁴. The publication layout gets names; `observation.ts` decodes them.

## What you can run or see

The `/lab/weapons` panel lists the live feed, and a decoded-frame inspector shows the new fields.

## Verification

- Native tests:
  - elevation and shot counters advance only on a real `prepare_launch`;
  - enemy weapon poses are absent unless identified (a metamorphic hidden-state test);
  - member ids stay stable across casualties, reinforcements and reacquisition;
  - counters stay exact past 2²⁴.
- `observation.test.ts` round-trips every new field.

Record frame cost in [`frame-cost.md`](../frame-cost.md) for this slice.

## Decision budget

- **Delegated:** Packing internals and field order.
- Anything else this slice has to invent is a spec gap. Record it in [`choices.md`](../choices.md) and resolve it here; don't improvise.

## Must stay green

Replay and digest parity. Every existing scene and test, except the pixel checks this slice retunes: each retune is recorded in `decisions.md`, never loosened silently. `bun run check` and `bun run verify` at closeout.

## Feedback that changes this slice

If publication bytes per tick at 100 a side grow by more than about 2×, reslice a delta encoding for weapon poses.
