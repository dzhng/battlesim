# 14 — Resupply sink

**Status:** planned. **Depends on:** 03. **Owns:** D10, D13 (hook), D27.

## Contract

A helicopter idle inside a deployed supply truck's zone (status `Serving`, `NoStock` or `Full`) sinks to the low hover and is served. It climbs back on any order or engagement. Resupply logic is unchanged.

## API seam

- `step_aircraft` reads the unit's `ServiceStatus` and sets `agl_target` to the derived low-hover height (D27).
- `supply.rs` is not changed.
- Expose `air::low_hover` as the hook [transport](../../transport/README.md) reuses for landing.

## What you can run or see

`cargo test -p sim --test supply`, and the `air` scene with a supply truck.

## Verification

Tests:
- `a_helicopter_sinks_at_a_deployed_truck_and_is_served`
- `it_climbs_on_an_order`
- `it_climbs_on_engagement`
- `sinking_never_flips_service_to_moving`

**Visual variable:** the sink
**Crop or mask:** 2× crop of the helicopter and the truck
**Out of scope (later slices):** none

1. Last check: run [screenshot-critique](../../../.agents/skills/screenshot-critique/SKILL.md) on the full frames and crops. Give the unprimed agent only the shots and a neutral task. Record its actionable findings before you accept the slice.

## Delegated to the implementer

Sink and climb rates.

Any other choice is a spec gap. Record it in [choices](../choices.md) and settle it before you widen the slice.

## Must stay green

Existing replay digests and `menu_reel` stay unchanged unless this slice names a digest change. Every test this slice touches must pass.

## Feedback that would change this slice

None expected.
