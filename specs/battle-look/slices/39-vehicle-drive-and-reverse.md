# 39 — Sim: tracked vs wheeled steering, and reverse

**Status:** planned. **Depends on:** 34c (the jeep and the push contact exist; the movement code is shared). **Lane:** simulation + controls. **Given:** [`movement-unknowns-map.html`](../movement-unknowns-map.html) (Q29–Q31).

## Contract

Today every vehicle turns like a tank. Beyond 60° of heading error it stops and pivots, at one shared `vehicle_turn_deg_s` of 45°/s, and nothing drives backwards (`movement.rs:266–276`). Now:
- **Q29, drive type is a property.** A tracked vehicle (the tank) pivots on the spot. A wheeled one (the truck, the jeep) has a minimum turning radius and never pivots. To turn around it drives a U-turn arc where there is room, otherwise an automatic three-point turn using reverse.
- **Q30, reverse is slower** than forward, as a per-vehicle fraction.
- **Q31, a reverse command.** **R** arms a reverse move (user 2026-09-26). Attack-move moves from R to **X**, and Ctrl+right-click still attack-moves straight away. With exactly one vehicle selected, a right-click in a small zone behind it is a reverse move instead of a normal move. The vehicle keeps its facing and backs along its route.

## API seam

- Fixture `bodies.<kind>`: `drive` (`tracked` or `wheeled`), `turn_deg_s`, `turning_radius_m` (wheeled) and `reverse_speed_fraction`. Defaults: tank tracked at 45°/s and 0.4×; truck wheeled at about 9 m and 0.35×; jeep wheeled at about 6 m and 0.4×. The shared `vehicle_turn_deg_s` is deleted.
- Movement: `step_vehicle` follows the route with the vehicle's own kinematics. Wheeled vehicles limit curvature to 1/radius, and plan an arc or a three-point turn when the next waypoint needs more. Route smoothing keeps its clearance; the follower owns the kinematics (pragmatism rule: no kinodynamic planner).
- Commands: `MoveOrder` gains `direction: Forward | Reverse`. A reverse move drives backwards along the route at the reverse speed, facing held. The digest covers it.
- Controls (`web/src/battle/input` `CommandBindings`): **R** arms reverse-move and **X** arms attack-move, a hard cutover. Update the readouts scene and the command bar labels. Implicit reverse is a right-click inside the zone behind a single selected vehicle, which is the strip behind the hull up to `controls.reverse_zone_length_m` (30) long and hull width + `controls.reverse_zone_margin_m` (2 per side) wide. With several units selected it is always a normal move.
- Slice 35 shows a reversing unit's final marker with its held facing and a reverse indicator.

## Verification

- Scenario runner (slice 30), reviewed by the agent:
  - a tank pivots 180° in place;
  - a truck U-turns in the open;
  - a truck does a three-point turn in a narrow lane;
  - a tank reverses out of a gap;
  - a reverse move is slower than the same move forwards.
- Native tests:
  - a wheeled vehicle never turns tighter than its radius, and never turns while stationary except as part of a three-point turn;
  - reverse speed equals the fraction times forward;
  - a reverse order holds facing;
  - replay parity.
- Controls scene:
  - R then a right-click issues a reverse move, and X then a right-click issues an attack-move;
  - a right-click in the zone behind a single selected tank issues a reverse move;
  - the same click with two vehicles selected issues a normal move;
  - a click outside the zone issues a normal move.
- Paired village and endurance reports (step instructions), logged.

## Decision budget

- **Delegated:** the arc and three-point-turn construction, and the numbers within the fixture.
- Anything else this slice has to invent is a spec gap. Record it in [`choices.md`](../choices.md) and resolve it here; don't improvise.

## Must stay green

Replay and digest parity (digests change by design; record it). Every existing scene and test. `bun run check` and `bun run verify` at closeout.
