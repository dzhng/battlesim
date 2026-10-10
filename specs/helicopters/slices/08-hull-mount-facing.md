# 08 — Hull-mount facing for every unit

**Status:** done. **Depends on:** 03, 07. **Owns:** D8, D28, D42, L4.

## Contract

A weapon fixed to the hull fires only when the body faces the target (bearing tolerance). A helicopter turns its airframe toward a target while it keeps flying its route. A ground vehicle with a hull mount turns only when it has no move order: tracked vehicles pivot, wheeled ones hold fire (D42). Turrets and infantry are unchanged.

## API seam

- In `weapons::point_mount`, a non-turret mount follows the body yaw and is no longer aimed instantly.
- The tolerance check (`weapons.rs`, the `TurretTraversing` reason) covers every non-infantry mount.
- New `Reach.face: Option<f64>`, which movement consumes: `step_aircraft` sets `aim_yaw` from it, and a tracked vehicle with no orders pivots through `turn_to`.
- Fixture: `test_attack_heli` (attack profile from D5: autocannon chin turret, and rockets on a hull mount once slice 09 lands), plus a test-local hull-mounted tracked unit.

## What you can run or see

`cargo test -p sim --test weapons`.

## Verification

Tests:
- `a_hull_mounted_gun_holds_fire_until_the_body_faces`
- `a_helicopter_strafes_facing_away_from_its_route`
- `a_tracked_hull_mount_pivots_only_without_a_move_order`
- `a_wheeled_hull_mount_holds_fire`
- every existing replay digest unchanged (Confirmed 4: no roster vehicle has a hull mount)

## Delegated to the implementer

Yaw rate when aiming versus when travelling.

Any other choice is a spec gap. Record it in [choices](../choices.md) and settle it before you widen the slice.

## Must stay green

Existing replay digests and `menu_reel` stay unchanged unless this slice names a digest change. Every test this slice touches must pass.

## Feedback that would change this slice

If the user dislikes helicopters strafing sideways, D8's helicopter half changes.
