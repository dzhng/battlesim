# 04 — Falling airframe (simulation)

**Status:** done in the sim (publication deferred to slice 10; see choices). **Depends on:** 01, 03. **Owns:** D3, D31, L2 (simulation).

## Contract

A helicopter shot down at altitude falls in a spinning arc that keeps its momentum. It bounces off a building, and on impact:
- fells every tree under it;
- does moderate splash damage to units and props beneath;
- leaves a `light_wreck` resting on the ground.

The whole fall is deterministic and in the digest.

## API seam

- New `sim::crash`, holding `Crash { velocity, spin_rad_s, started }` per downed airframe, digested in `Battle::digest`.
- An airborne death goes here instead of to `drive::death_roll` and the immediate `add_prop` with `base_z: Some(position.z)` (`battle.rs`).
- Impact fires the new data row `helicopter_crash` through `damage::blast` and the prop blast path, so trees fall through `destroy_prop`. Then it places the wreck at ground z.
- D31: the falling airframe isn't a body; a wreck over a live hull moves to the nearest spot clear of hulls; credit goes to the shooter.
- New publication group `crashes` (id, x, y, z, yaw, pitch, roll), shown only to sides that saw the death, by the same rule wrecks use.

## What you can run or see

`cargo test -p sim --test air` (fall cases).

## Verification

Tests:
- `a_shot_down_helicopter_falls_forward_and_spins`
- `the_wreck_rests_on_the_ground_not_in_the_air`
- `its_crash_fells_every_tree_under_it`
- `it_bounces_off_a_building`
- `units_under_it_take_moderate_damage`
- `a_tank_pushes_the_wreck`
- `crash_kills_are_credited_to_the_shooter`
- a twice-run digest match

`helicopter_crash` row values: structural damage ≥ 100, the trunk's HP; unit damage below the matching `tank_he` row.

## Delegated to the implementer

Restitution of the bounce, as long as it bounces at most once; spin rate; the `helicopter_crash` radius within the airframe's footprint.

Any other choice is a spec gap. Record it in [choices](../choices.md) and settle it before you widen the slice.

## Must stay green

Existing replay digests and `menu_reel` stay unchanged unless this slice names a digest change. Every test this slice touches must pass.

## Feedback that would change this slice

If the fall looks too long or too short on screen in slice 10, retune the arc's drag.
