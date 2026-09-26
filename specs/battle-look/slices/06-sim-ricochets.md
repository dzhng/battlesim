# 06 — Sim: ricochets

**Status:** done (2026-09-25). **Depends on:** 05. **Lane:** simulation.

## Contract

A kinetic round that fails to penetrate may ricochet: a per-face probability (front, side, rear, roof) from the unit's armour table, independent of any 3D model (Q9). It deflects off the simulation's hull box with reduced speed and scatter, and flies on as a real round, at most 2 bounces and with reduced penetration. HE always detonates.

## API seam

- `flight::fly_tick` takes an `ImpactResolver(ImpactContext) -> ImpactDecision {Stop, Detonate, Bounce{velocity}}`.
  - `damage` owns penetration and face policy once.
  - The context uses the hit-time `Body::pose_at`, not the end-of-tick pose (landmine 8).
  - A named, seeded `Rng` stream.
  - Re-fly the rest of the tick from the impact, with a fresh broadphase query that excludes the struck hull.
  - `FlightEvent::Ricochet` keeps the same `ProjectileId`. `flight_load.rs:157` still pins one ending per round.
  - The bounce count joins the projectile digest.
- Fixture `health.<kind>_armor.ricochet {front, side, rear, roof}`, plus `ricochet.speed_kept`, `scatter_deg`, `penetration_kept` and `max_bounces: 2`.
- `VisibleSegment` becomes a polyline per round per tick. Enemy segments are clipped piece by piece by `clip_to_seen`.

## What you can run or see

`/lab/ballistics` gains an oblique-AP preset showing bounce paths, plus a native ricochet trace CLI.

## Verification

**Kill gate first**, before the full slice: worker and direct runs give the same digests, and endurance tick p95 rises no more than 10%.

**Fallback:** a next-tick child round, a named deviation that needs the user.

Native tests:
- the seeded bounce distribution per face;
- never more than 2 bounces;
- HE never bounces;
- a second collider hit after a bounce;
- near-miss suppression deduplication across the bounce;
- penetration reduced after a bounce;
- digest parity.

Record frame cost in [`frame-cost.md`](../frame-cost.md) for this slice.

## Decision budget

- **Delegated:** Scatter model, and the fixture probabilities within their provisional range.
- Anything else this slice has to invent is a spec gap. Record it in [`choices.md`](../choices.md) and resolve it here; don't improvise.

## Must stay green

Replay and digest parity. Every existing scene and test, except the pixel checks this slice retunes: each retune is recorded in `decisions.md`, never loosened silently. `bun run check` and `bun run verify` at closeout.

## Feedback that changes this slice

If probabilities make tank duels degenerate in `village_report`, re-tune them with paired seeds.
